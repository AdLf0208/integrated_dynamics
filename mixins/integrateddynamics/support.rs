// Game-side glue for Integrated Dynamics slice 1 (injected into `crate::game`
// as `integrateddynamics_state`; it needs the game, so it is not part of the
// standalone library).
//
// Upstream a Logic Cable is a multipart block entity: each of its six faces can
// hold one part, and every part's aspect reads or writes a value on the network.
// This slice keeps the six parts as the cable's `part_<face>` state properties
// (so the model can show them and they persist without a block-entity write),
// keeps the network channel value in the cable's saved compound, and after the
// level's tick recomputes every connected network from the cables that ticked
// this tick: the channel is the strongest Redstone Reader input, and every
// Redstone Writer on the network emits it through `native_block_signal`.
//
// A block entity's own tick must not write its NBT (the engine reverts it), so
// the tick only queues the cable and the writes happen at `level_tick_post`.
// The channel is recomputed once per level tick: a handful of cables makes the
// full network walk cheaper than tracking a dirty graph across ticks.

use std::collections::BTreeSet;
use std::sync::Mutex;

use horizon_client::containers::{BlockEntity, ModBlockEntity};
use horizon_client::text::Text;
use horizon_core::position::BlockPos;
use horizon_world::nbt::{Compound, JavaString, Tag};
use horizon_world::registry::{BlockRegistry, BlockStateId};

use crate::game::integrateddynamics::{
    FACE_COUNT, FACE_NAMES, Part, Parts, channel_value, collect_network, face_of_normal,
    is_signal_source, signal_for_face, status_for,
};
use crate::game::native_hooks::flags;
use crate::game::native_server::{NativeCommand, NativeServerPlayer};

/// The Logic Cable block (and its item).
pub(super) const CABLE: &str = "integrateddynamics:cable";
/// The Redstone Reader part item.
pub(super) const READER: &str = "integrateddynamics:part_redstone_reader";
/// The Redstone Writer part item.
pub(super) const WRITER: &str = "integrateddynamics:part_redstone_writer";
/// The cable block entity's saved channel value.
const VALUE_TAG: &str = "value";
/// A safety cap on one network's cable count (upstream has no limit; a runaway
/// build must not stall the level tick).
const MAX_NETWORK: usize = 1024;

#[derive(Default)]
struct State {
    /// Cables whose block entity ticked this tick, recomputed at the level
    /// tick's end.
    cables: BTreeSet<[i32; 3]>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn state() -> std::sync::MutexGuard<'static, Option<State>> {
    STATE.lock().unwrap_or_else(|poison| poison.into_inner())
}

fn key(name: &str) -> JavaString {
    JavaString::from(name)
}

fn int(nbt: &Compound, name: &str) -> Option<i32> {
    match nbt.get(&key(name)) {
        Some(Tag::Int(value)) => Some(*value),
        _ => None,
    }
}

/// The part property of `face` (`part_down` … `part_east`).
fn face_property(face: usize) -> String {
    format!("part_{}", FACE_NAMES[face])
}

/// The six parts a cable state carries, or `None` for another block.
fn parts_of(registry: &BlockRegistry, state: BlockStateId) -> Option<Parts> {
    let state = registry.state(state)?;
    if &*state.name != CABLE {
        return None;
    }
    let mut parts = Parts::new();
    for face in 0..FACE_COUNT {
        let value = state.properties.get(&face_property(face))?;
        parts = parts.with(face, Part::from_name(value)?);
    }
    Some(parts)
}

/// The parts of the cable at `pos` in the level in place.
fn parts_at(game: &crate::game::Game<'_>, pos: [i32; 3]) -> Option<Parts> {
    if game.native_block(pos) != CABLE {
        return None;
    }
    let mut parts = Parts::new();
    for face in 0..FACE_COUNT {
        let value = game.native_block_property(pos, &face_property(face))?;
        parts = parts.with(face, Part::from_name(&value)?);
    }
    Some(parts)
}

/// The channel value saved in a cable block entity, 0 when there is none.
fn value_at(world: &horizon_client::world::World, pos: [i32; 3]) -> u8 {
    let Some(BlockEntity::Mod(entity)) = world.block_entities.get(&(pos[0], pos[1], pos[2]))
    else {
        return 0;
    };
    if entity.id != CABLE {
        return 0;
    }
    int(&entity.nbt, VALUE_TAG).unwrap_or(0).clamp(0, 15) as u8
}

/// Save a cable's channel value; answers whether it changed.
fn set_value(game: &mut crate::game::Game<'_>, pos: [i32; 3], value: u8) -> bool {
    let Some(BlockEntity::Mod(entity)) =
        game.world.block_entities.get_mut(&(pos[0], pos[1], pos[2]))
    else {
        return false;
    };
    if entity.id != CABLE {
        return false;
    }
    if int(&entity.nbt, VALUE_TAG) == Some(i32::from(value)) {
        return false;
    }
    entity.nbt.insert(key(VALUE_TAG), Tag::Int(i32::from(value)));
    game.world.modified.insert((pos[0] >> 4, pos[2] >> 4));
    true
}

/// Show the network's part status on a cable (the writer model's
/// active/inactive texture); a no-op when it is already right.
fn show_status(game: &mut crate::game::Game<'_>, pos: [i32; 3], parts: Parts, value: u8) {
    let status = status_for(parts, value).name();
    if game.native_block_property(pos, "status").as_deref() != Some(status) {
        game.native_set_block_property_with(pos, "status", status, flags::UPDATE_CLIENTS);
    }
}

/// A placed cable gets a block entity for its channel value (`native_block_entity`,
/// API 15).
#[allow(private_interfaces)]
pub(super) fn on_block_entity(block: &str, entity: &mut Option<(String, Compound)>) {
    if block == CABLE && entity.is_none() {
        *entity = Some((CABLE.to_string(), Compound::new()));
    }
}

/// Every cable ticks on the server (`native_block_entity_ticks`, API 15).
#[allow(private_interfaces)]
pub(super) fn on_block_entity_ticks(block: &str, ticks: &mut bool) {
    if block == CABLE {
        *ticks = true;
    }
}

/// One cable tick: queue the cable for the level tick's end. The tick itself
/// writes nothing (the engine reverts block-entity writes made here).
#[allow(private_interfaces)]
pub(super) fn on_block_entity_tick(
    game: &crate::game::Game<'_>,
    pos: [i32; 3],
    _entity: &ModBlockEntity,
) {
    if game.native_block(pos) == CABLE {
        // The first tick creates the per-tick set; a stale one never is.
        state().get_or_insert_with(State::default).cables.insert(pos);
    }
}

/// After the level's tick: recompute every queued cable network. The channel is
/// the strongest reader input on the network, stored on every cable in it.
#[allow(private_interfaces)]
pub(super) fn on_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    let queued = {
        let mut guard = state();
        let cables = guard.as_mut().map(|state| std::mem::take(&mut state.cables));
        cables.unwrap_or_default()
    };
    if queued.is_empty() {
        return;
    }
    let mut done: BTreeSet<[i32; 3]> = BTreeSet::new();
    for start in queued {
        if done.contains(&start) || game.native_block(start) != CABLE {
            continue;
        }
        let network = collect_network(start, |pos| game.native_block(pos) == CABLE, MAX_NETWORK);
        let mut readings = Vec::new();
        for &pos in &network {
            if let Some(parts) = parts_at(game, pos) {
                if parts.reader_faces().next().is_some() {
                    readings.push(game.native_neighbor_signal(dimension, pos));
                }
            }
        }
        let value = channel_value(&readings);
        for &pos in &network {
            done.insert(pos);
            let Some(parts) = parts_at(game, pos) else {
                continue;
            };
            let changed = set_value(game, pos, value);
            show_status(game, pos, parts, value);
            // A changed answer notifies the neighbours; while the cable emits,
            // it re-notifies each tick so a consumer that missed the change
            // (a lamp toggling on its own delayed tick) reads it again.
            if changed || (parts.has_writer() && value > 0) {
                game.native_update_signal_neighbors(dimension, pos);
            }
        }
    }
}

/// `block_signal`: a writer face answers the cable's channel value as a weak
/// signal; every other face answers 0. No strong (direct) output, as upstream's
/// writer aspect defaults to `direct = false` (`native_block_signal`, API 38).
#[allow(private_interfaces)]
pub(super) fn on_block_signal(
    level: &crate::game::GameLevel<'_>,
    pos: BlockPos,
    state: BlockStateId,
    face: horizon_core::geometry::Direction,
    direct: bool,
    signal: &mut i32,
) {
    let Some(parts) = parts_of(level.registry, state) else {
        return;
    };
    let value = value_at(&level.world, [pos.x, pos.y, pos.z]);
    // The hook's `face` is the side that touches the block reading it (the
    // emitting face), as `native_block_signal` documents.
    *signal = signal_for_face(parts, value, face_of_normal(face.normal()).unwrap_or(0), direct);
}

/// `is_signal_source`: a cable with a writer is one, so redstone wire beside it
/// connects (`native_is_signal_source`, API 38).
#[allow(private_interfaces)]
pub(super) fn on_is_signal_source(
    level: &crate::game::GameLevel<'_>,
    state: BlockStateId,
    source: &mut bool,
) {
    if parts_of(level.registry, state).is_some_and(is_signal_source) {
        *source = true;
    }
}

/// Right-clicking a cable face with a reader or writer places that part on the
/// face (upstream's `ItemPart.useOn` + `PartHelpers.addPart`); the item is
/// consumed outside creative. A face that already holds a part is left for the
/// (unported) screen.
#[allow(private_interfaces)]
pub(super) fn on_right_click_block(
    game: &mut crate::game::Game<'_>,
    pos: [i32; 3],
    face_normal: [i32; 3],
    canceled: &mut bool,
) {
    if game.native_block(pos) != CABLE {
        return;
    }
    let Some(face) = face_of_normal(face_normal) else {
        return;
    };
    let Some((held, _, _)) = game.native_held() else {
        return;
    };
    let part = match held.as_str() {
        READER => Part::Reader,
        WRITER => Part::Writer,
        _ => return,
    };
    let property = face_property(face);
    if game.native_block_property(pos, &property).as_deref() != Some("none") {
        *canceled = true;
        game.native_feedback(Text::literal(format!(
            "Integrated Dynamics: this cable face already holds a part ({})",
            FACE_NAMES[face]
        )));
        return;
    }
    game.native_set_block_property_with(pos, &property, part.name(), flags::UPDATE_CLIENTS);
    if game
        .native_acting_player()
        .is_none_or(|player| player.game_mode != 1)
    {
        game.native_take_items(&held, 1);
    }
    *canceled = true;
    game.native_feedback(Text::literal(format!(
        "Integrated Dynamics: {} on the {} side",
        part.name(),
        FACE_NAMES[face]
    )));
}

/// Declare `/integrateddynamics info|part`.
#[allow(private_interfaces)]
pub(super) fn on_register_commands(commands: &mut Vec<NativeCommand>) {
    commands.push(NativeCommand::new("integrateddynamics info <pos:block_pos>"));
    commands.push(NativeCommand::new(
        "integrateddynamics part <pos:block_pos> <face:word> <part:word>",
    ));
    commands.push(NativeCommand::new("integrateddynamics probe <pos:block_pos>"));
}

/// The parts of a cable as `down=none,north=reader`.
fn parts_line(parts: Parts) -> String {
    (0..FACE_COUNT)
        .map(|face| {
            format!(
                "{}={}",
                FACE_NAMES[face],
                parts.get(face).unwrap_or(Part::None).name()
            )
        })
        .collect::<Vec<_>>()
        .join(",")
}

fn parse_pos(words: &[&str; 3]) -> Option<[i32; 3]> {
    Some([
        words[0].parse().ok()?,
        words[1].parse().ok()?,
        words[2].parse().ok()?,
    ])
}

/// Run `/integrateddynamics info <x> <y> <z>` (the harness readout: parts,
/// channel value and status) or `/integrateddynamics part <x> <y> <z> <face>
/// <none|reader|writer>` (the harness path for placing/removing a part, since
/// `ctl interact` only reaches a visible face).
#[allow(private_interfaces)]
pub(super) fn on_perform_command(
    game: &mut crate::game::Game<'_>,
    _player: &NativeServerPlayer,
    command: &str,
    ran: &mut bool,
) {
    let words: Vec<&str> = command.split_whitespace().collect();
    match words.as_slice() {
        ["integrateddynamics", "info", x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal(
                    "Usage: /integrateddynamics info <x> <y> <z>",
                ));
                return;
            };
            let Some(parts) = parts_at(game, pos) else {
                game.native_feedback(Text::literal("Integrated Dynamics: no Logic Cable there."));
                return;
            };
            game.native_feedback(Text::literal(format!(
                "Logic Cable at {x},{y},{z}: parts {}",
                parts_line(parts)
            )));
            let value = value_at(&game.world, pos);
            game.native_feedback(Text::literal(format!(
                "Logic Cable at {x},{y},{z}: value {value}; status {}",
                status_for(parts, value).name()
            )));
        }
        ["integrateddynamics", "part", x, y, z, face, part] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal(
                    "Usage: /integrateddynamics part <x> <y> <z> <face> <none|reader|writer>",
                ));
                return;
            };
            let Some(face) = FACE_NAMES.iter().position(|name| name == face) else {
                game.native_feedback(Text::literal(format!(
                    "Integrated Dynamics: unknown face '{face}' (down|up|north|south|west|east)"
                )));
                return;
            };
            if game.native_block(pos) != CABLE {
                game.native_feedback(Text::literal("Integrated Dynamics: no Logic Cable there."));
                return;
            }
            let Some(part) = Part::from_name(part) else {
                game.native_feedback(Text::literal(format!(
                    "Integrated Dynamics: unknown part '{part}' (none|reader|writer)"
                )));
                return;
            };
            let property = face_property(face);
            game.native_set_block_property_with(pos, &property, part.name(), flags::UPDATE_CLIENTS);
            game.native_feedback(Text::literal(format!(
                "Logic Cable at {x},{y},{z}: {} on the {} side",
                part.name(),
                FACE_NAMES[face]
            )));
        }
        ["integrateddynamics", "probe", x, y, z] => {
            *ran = true;
            let Some(pos) = parse_pos(&[*x, *y, *z]) else {
                game.native_feedback(Text::literal(
                    "Usage: /integrateddynamics probe <x> <y> <z>",
                ));
                return;
            };
            let dimension = game.native_dimension();
            let signal = game.native_neighbor_signal(&dimension, pos);
            game.native_feedback(Text::literal(format!(
                "Probe at {x},{y},{z}: neighbour signal {signal}"
            )));
        }
        ["integrateddynamics", ..] => {
            *ran = true;
            game.native_feedback(Text::literal(
                "Usage: /integrateddynamics info <x> <y> <z> | part <x> <y> <z> <face> <part>",
            ));
        }
        _ => {}
    }
}
