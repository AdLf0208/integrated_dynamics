// Game-side glue for Integrated Dynamics slice 1.
//
// Upstream a Logic Cable is a multipart block entity: each of its six faces can
// hold one part, and every part's aspect reads or writes a value on the network.
// This slice keeps the six parts as the cable's `part_<face>` state properties
// (so the model can show them and they persist without a block-entity write),
// keeps the network channel value in the cable's saved compound, and after the
// level's tick recomputes every connected network from the cables that ticked
// this tick: the channel is the strongest Redstone Reader input, and every
// Redstone Writer on the network emits it from its own face.
//
// A block entity's own tick only queues the cable; the writes happen at
// `level_tick_post`, where a block entity is back in the world. The channel is
// recomputed once per level tick: a handful of cables makes the full network
// walk cheaper than tracking a dirty graph across ticks.
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;

use mod_api::{
    BlockEntityEvent, BlockEntityTickEvent, BlockKey, BlockSignalEvent, BlockState, Command,
    CommandArgumentValue, CommandInvocation, Compound, Direction, Flow, Host, HostTransaction,
    InteractionResult, IsSignalSourceEvent, ItemPlace, ItemResource, LevelHost, LevelRef,
    LevelSide, NeighbourUpdate, RegisterCommandsEvent, RunModCommandEvent, ServerHost, Tag, Text,
    UseItemOnEvent,
};

use crate::CABLE;
use crate::runtime::{
    FACE_COUNT, FACE_NAMES, Part, Parts, Pos, channel_value, collect_network, face_of_normal,
    is_signal_source, neighbours, signal_for_face, status_for,
};

/// The cable block entity's saved channel value.
const VALUE_TAG: &str = "value";
/// A safety cap on one network's cable count (upstream has no limit; a runaway
/// build must not stall the level tick).
const MAX_NETWORK: usize = 1024;

/// What the mod keeps between calls. A Part is a separate object and a hook's
/// `host` is the game, so the cable's cross-call state lives here.
#[derive(Default)]
struct State {
    /// Cables whose block entity ticked this tick, recomputed at the level
    /// tick's end, by dimension name (`minecraft:overworld`).
    queued: HashSet<(&'static str, Pos)>,
    /// The channel value each cable last answered, for `block_signal`: that hook
    /// is asked very often and must not read the world, so it reads this
    /// instead. The saved compound stays the truth a reload restores from.
    values: HashMap<(&'static str, Pos), u8>,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

/// Run one short call against the cable state, creating it on first use.
fn with_state<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut guard = STATE
        .lock()
        .expect("the cable state lock is held for one short call");
    f(guard.get_or_insert_with(State::default))
}

/// A cable state, by its block and local ordinal.
type StateKey = (BlockKey, u16);
/// The parts each cable state carries; `None` for a state that is not a cable.
type PartsCache = HashMap<StateKey, Option<Parts>>;

/// Each cable state's parts, computed once: a state's properties never change,
/// so one parse per state is enough.
static PARTS: Mutex<Option<PartsCache>> = Mutex::new(None);

/// Run one short call against the state cache, creating it on first use.
fn with_parts<R>(f: impl FnOnce(&mut PartsCache) -> R) -> R {
    let mut guard = PARTS
        .lock()
        .expect("the cable state cache lock is held for one short call");
    f(guard.get_or_insert_with(HashMap::new))
}

fn parts_cached(state: BlockState) -> Option<Option<Parts>> {
    with_parts(|cache| cache.get(&(state.block(), state.ordinal())).copied())
}

fn parts_remember(state: BlockState, parts: Option<Parts>) -> Option<Parts> {
    with_parts(|cache| cache.insert((state.block(), state.ordinal()), parts));
    parts
}

/// The six parts a state carries, read through a host's state table.
fn parts_of_state(host: &dyn Host, state: BlockState) -> Option<Parts> {
    if let Some(cached) = parts_cached(state) {
        return cached;
    }
    let parts = host
        .block_state_string(state)
        .as_deref()
        .and_then(parts_from_state_string);
    parts_remember(state, parts)
}

/// The same, through a level's read-only host (the block-signal hooks, which are
/// given no writable host).
fn parts_in_level(level: &dyn LevelHost, state: BlockState) -> Option<Parts> {
    if let Some(cached) = parts_cached(state) {
        return cached;
    }
    let parts = level
        .state_string(state)
        .as_deref()
        .and_then(parts_from_state_string);
    parts_remember(state, parts)
}

/// The six parts in a serialized state, `/setblock`'s form:
/// `integrateddynamics:cable[part_down=none,…,status=active]`. `None` for any
/// other block, a state without its six part properties, or an unknown value.
fn parts_from_state_string(text: &str) -> Option<Parts> {
    let (name, rest) = text.split_once('[')?;
    if name != CABLE {
        return None;
    }
    let properties = rest.strip_suffix(']')?;
    let mut parts = Parts::new();
    let mut seen = 0;
    for entry in properties.split(',') {
        let (key, value) = entry.split_once('=')?;
        let Some(name) = key.strip_prefix("part_") else {
            continue;
        };
        let face = FACE_NAMES.iter().position(|known| *known == name)?;
        parts = parts.with(face, Part::from_name(value)?);
        seen += 1;
    }
    (seen == FACE_COUNT).then_some(parts)
}

/// The parts of the cable at `pos`, or `None` for another block.
fn parts_at(host: &dyn Host, level: LevelRef, pos: Pos) -> Option<Parts> {
    let state = level.block_state(host, pos)?;
    parts_of_state(host, state)
}

/// Whether the block at `pos` is a Logic Cable.
fn is_cable(host: &dyn Host, level: LevelRef, pos: Pos) -> bool {
    level
        .block(host, pos)
        .is_some_and(|block| block.id(host) == CABLE)
}

/// The part property of `face` (`part_down` … `part_east`).
fn face_property(face: usize) -> String {
    format!("part_{}", FACE_NAMES[face])
}

/// A placed cable gets a block entity for its channel value
/// (`EntityBlock.newBlockEntity`).
pub(crate) fn on_block_entity(
    _level: LevelSide,
    event: &mut BlockEntityEvent<'_>,
) -> Option<Compound> {
    (event.block == CABLE).then(Compound::new)
}

/// One cable tick: queue the cable for the level tick's end. The tick itself
/// writes nothing; the network is recomputed where the block entity is back in
/// the world.
pub(crate) fn on_block_entity_tick(
    host: &mut dyn ServerHost,
    event: &mut BlockEntityTickEvent<'_>,
) {
    let dimension = host.dimension();
    with_state(|state| state.queued.insert((dimension, event.pos)));
}

/// After the level's tick: recompute every queued cable network. The channel is
/// the strongest reader input on the network, stored on every cable in it.
pub(crate) fn on_level_tick_post(host: &mut dyn ServerHost, level: LevelRef) {
    let dimension = host.dimension();
    let queued: Vec<Pos> = with_state(|state| {
        let this_level: Vec<Pos> = state
            .queued
            .iter()
            .filter(|(queued_dimension, _)| *queued_dimension == dimension)
            .map(|(_, pos)| *pos)
            .collect();
        for pos in &this_level {
            state.queued.remove(&(dimension, *pos));
        }
        this_level
    });
    if queued.is_empty() {
        return;
    }
    let mut done: HashSet<Pos> = HashSet::new();
    for start in queued {
        // The reads borrow the host immutably; the writes below need it mutably,
        // so each network's plan is complete before anything is written.
        let (network, readings) = {
            let read: &dyn Host = &*host;
            if done.contains(&start) || !is_cable(read, level, start) {
                continue;
            }
            let walk = |pos: Pos| is_cable(read, level, pos);
            let network = collect_network(start, walk, MAX_NETWORK);
            let mut readings = Vec::new();
            for &pos in &network {
                let Some(parts) = parts_at(read, level, pos) else {
                    continue;
                };
                for face in parts.reader_faces() {
                    // The reader samples the block on its own face, as upstream's
                    // `getSignal(targetPos, targetSide)` does: the target's signal
                    // toward the cable.
                    let target = neighbours(pos)[face];
                    let toward_cable = Direction::ALL[face].opposite();
                    readings.push(level.signal(read, target, toward_cable).unwrap_or(0));
                }
            }
            (network, readings)
        };
        let value = channel_value(&readings);
        for pos in network {
            done.insert(pos);
            let Some(parts) = parts_at(host, level, pos) else {
                continue;
            };
            let changed = set_value(host, level, pos, value);
            // `set_block_property` is a no-op when the property already holds the
            // value, so the status needs no read of its own.
            show_status(host, pos, parts, value);
            with_state(|state| state.values.insert((dimension, pos), value));
            // A changed answer notifies the neighbours; while the cable emits,
            // it re-notifies each tick so a consumer that missed the change
            // (a lamp toggling on its own delayed tick) reads it again.
            if (changed || (parts.has_writer() && value > 0))
                && let Some(source) = level.block_state(host, pos)
            {
                let _ = level.update_neighbors(host, pos, source, NeighbourUpdate::Neighbours);
            }
        }
    }
}

/// A level closed: drop what was kept for it (its queued ticks and its channel
/// values). The saved compounds keep the values a reload restores from.
pub(crate) fn on_level_unload(dimension: &str) {
    with_state(|state| {
        state.queued.retain(|(kept, _)| *kept != dimension);
        state.values.retain(|(kept, _), _| *kept != dimension);
    });
}

/// Save a cable's channel value; answers whether it changed.
fn set_value(host: &mut dyn ServerHost, level: LevelRef, pos: Pos, value: u8) -> bool {
    let Some(entity) = host.block_entity(level, pos) else {
        return false;
    };
    let mut data = Tag::Compound(entity.data(host).unwrap_or_default());
    if matches!(data.get(VALUE_TAG), Some(Tag::Int(current)) if *current == i32::from(value)) {
        return false;
    }
    let Tag::Compound(entries) = &mut data else {
        return false;
    };
    entries.insert(VALUE_TAG.into(), Tag::Int(i32::from(value)));
    let Tag::Compound(entries) = data else {
        return false;
    };
    entity.merge_data(host, entries).unwrap_or(false)
}

/// Show the network's part status on a cable (the writer model's
/// active/inactive texture).
fn show_status(host: &mut dyn ServerHost, pos: Pos, parts: Parts, value: u8) {
    let _ = host.set_block_property(pos, "status", status_for(parts, value).name());
}

/// `block_signal`: a writer face answers the cable's channel value as a weak
/// signal; every other face answers 0. No strong (direct) output, as upstream's
/// writer aspect defaults to `direct = false`.
pub(crate) fn on_block_signal(level: &dyn LevelHost, event: &mut BlockSignalEvent) {
    let Some(parts) = parts_in_level(level, event.state) else {
        return;
    };
    let dimension = event.dimension.key().id();
    let pos = [event.pos.x, event.pos.y, event.pos.z];
    let value = with_state(|state| state.values.get(&(dimension, pos)).copied()).unwrap_or(0);
    // The hook's `face` is the side that touches the block reading it (the
    // emitting face), as `block_signal` documents.
    let Some(face) = face_of_normal(event.face.normal()) else {
        return;
    };
    event.signal = signal_for_face(parts, value, face, event.direct);
}

/// `is_signal_source`: a cable with a writer is one, so redstone wire beside it
/// connects.
pub(crate) fn on_is_signal_source(level: &dyn LevelHost, event: &mut IsSignalSourceEvent) {
    if parts_in_level(level, event.state).is_some_and(is_signal_source) {
        event.source = true;
    }
}

/// Clicking a cable face with a reader or writer places that part on the face
/// (upstream's `ItemPart.useOn`); the item is consumed outside creative. A face
/// that already holds a part is left for the (unported) screen.
pub(crate) fn on_use_item_on(
    host: &mut dyn ServerHost,
    event: &mut UseItemOnEvent<'_>,
) -> Option<InteractionResult> {
    let click = event.click;
    let stack = click.stack?;
    let held = stack.item().id(host);
    let part = match held.as_str() {
        crate::READER => Part::Reader,
        crate::WRITER => Part::Writer,
        _ => return None,
    };
    let face = face_of_normal(click.hit.face)?;
    if click.dimension.key().id() != host.dimension() {
        return None;
    }
    let level = host.level(host.dimension())?;
    let property = face_property(face);
    if level.block_property(host, click.pos, &property).as_deref() != Some("none") {
        // A face that holds a part is refused; the placed-part screen is out of
        // this slice, so the click ends without a second part.
        tell(
            host,
            click.player,
            format!(
                "Integrated Dynamics: this cable face already holds a part ({})",
                FACE_NAMES[face]
            ),
        );
        return Some(InteractionResult::Fail);
    }
    if !host.set_block_property(click.pos, &property, part.name()) {
        return None;
    }
    // A creative hand gives the item without losing it: the engine's own place
    // rules decide, so this is one extract, not a game-mode branch.
    let resource = ItemResource::of(stack);
    let place = ItemPlace::PlayerHand {
        player: click.player,
        hand: click.hand,
    };
    let _ = host.transaction(|host, tx| Ok::<u32, ()>(place.extract(host, tx, &resource, 1)));
    tell(
        host,
        click.player,
        format!(
            "Integrated Dynamics: {} on the {} side",
            part.name(),
            FACE_NAMES[face]
        ),
    );
    Some(InteractionResult::Success)
}

/// Declare `/integrateddynamics info|part|probe`.
pub(crate) fn on_register_commands(
    _host: &mut dyn ServerHost,
    event: &mut RegisterCommandsEvent<'_>,
) {
    event
        .commands
        .push(Command::new("integrateddynamics info <pos:block_pos>"));
    event.commands.push(Command::new(
        "integrateddynamics part <pos:block_pos> <face:word> <part:word>",
    ));
    event
        .commands
        .push(Command::new("integrateddynamics probe <pos:block_pos>"));
}

/// Run `/integrateddynamics info <x> <y> <z>` (the harness readout: parts,
/// channel value and status), `/integrateddynamics part <x> <y> <z> <face>
/// <none|reader|writer>` (the harness path for placing/removing a part, since a
/// `ctl interact` reaches only a visible face) or `/integrateddynamics probe
/// <x> <y> <z>` (the strongest signal reaching a position).
pub(crate) fn on_run_mod_command(
    host: &mut dyn ServerHost,
    event: &mut RunModCommandEvent<'_>,
) -> Flow {
    let invocation = event.command;
    let mut literals = invocation.literals.iter().map(String::as_str);
    if literals.next() != Some("integrateddynamics") {
        return Flow::Continue;
    }
    let Some(subcommand) = literals.next() else {
        return Flow::Continue;
    };
    let Some(dimension) = invocation.dimension.id(host) else {
        reply(host, "Integrated Dynamics: that dimension is not known.");
        return Flow::Cancel;
    };
    let Some(level) = host.level(&dimension) else {
        reply(host, "Integrated Dynamics: that dimension is not loaded.");
        return Flow::Cancel;
    };
    let Some(pos) = block_pos(invocation, "pos") else {
        reply(host, "Usage: /integrateddynamics <subcommand> <x> <y> <z>");
        return Flow::Cancel;
    };
    match subcommand {
        "info" => {
            let Some(parts) = parts_at(host, level, pos) else {
                reply(host, "Integrated Dynamics: no Logic Cable there.");
                return Flow::Cancel;
            };
            let [x, y, z] = pos;
            reply(
                host,
                &format!("Logic Cable at {x},{y},{z}: parts {}", parts_line(parts)),
            );
            let value = cable_value(host, level, pos);
            reply(
                host,
                &format!(
                    "Logic Cable at {x},{y},{z}: value {value}; status {}",
                    status_for(parts, value).name()
                ),
            );
        }
        "part" => {
            let Some(face) = word(invocation, "face")
                .and_then(|name| FACE_NAMES.iter().position(|k| *k == name))
            else {
                reply(
                    host,
                    "Integrated Dynamics: unknown face (down|up|north|south|west|east)",
                );
                return Flow::Cancel;
            };
            let Some(part) = word(invocation, "part").and_then(Part::from_name) else {
                reply(
                    host,
                    "Integrated Dynamics: unknown part (none|reader|writer)",
                );
                return Flow::Cancel;
            };
            // The write helpers act in the level in place, so a part is only set
            // in the level this command runs in (not through `execute in`).
            if dimension != host.dimension() {
                reply(
                    host,
                    "Integrated Dynamics: run this command in the cable's dimension.",
                );
                return Flow::Cancel;
            }
            let is_cable = level
                .block_state(host, pos)
                .and_then(|state| parts_of_state(host, state))
                .is_some();
            if !is_cable {
                reply(host, "Integrated Dynamics: no Logic Cable there.");
                return Flow::Cancel;
            }
            if host.set_block_property(pos, &face_property(face), part.name()) {
                let [x, y, z] = pos;
                reply(
                    host,
                    &format!(
                        "Logic Cable at {x},{y},{z}: {} on the {} side",
                        part.name(),
                        FACE_NAMES[face]
                    ),
                );
            } else {
                reply(host, "Integrated Dynamics: the part could not be set.");
            }
        }
        "probe" => {
            let signal = level.best_neighbor_signal(host, pos).unwrap_or(0);
            let [x, y, z] = pos;
            reply(
                host,
                &format!("Probe at {x},{y},{z}: neighbour signal {signal}"),
            );
        }
        _ => reply(
            host,
            "Usage: /integrateddynamics info <x> <y> <z> | part <x> <y> <z> <face> <part> | probe <x> <y> <z>",
        ),
    }
    Flow::Cancel
}

/// The channel value saved in a cable block entity, 0 when there is none.
fn cable_value(host: &dyn Host, level: LevelRef, pos: Pos) -> u8 {
    let Some(entity) = host.block_entity(level, pos) else {
        return 0;
    };
    let data = Tag::Compound(entity.data(host).unwrap_or_default());
    match data.get(VALUE_TAG) {
        Some(Tag::Int(value)) => (*value).clamp(0, 15) as u8,
        _ => 0,
    }
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

/// A `block_pos` argument by name.
fn block_pos(invocation: &CommandInvocation, name: &str) -> Option<Pos> {
    match invocation.argument(name) {
        Some(CommandArgumentValue::BlockPos(pos)) => Some(*pos),
        _ => None,
    }
}

/// A `word` argument by name.
fn word<'a>(invocation: &'a CommandInvocation, name: &str) -> Option<&'a str> {
    match invocation.argument(name) {
        Some(CommandArgumentValue::String(value)) => Some(value.as_str()),
        _ => None,
    }
}

/// Answer a command's source, as the console or chat would.
fn reply(host: &mut dyn ServerHost, line: &str) {
    let _ = host.commands().feedback(Text::literal(line));
}

/// A player's on-screen feedback for a click they made.
fn tell(host: &mut dyn ServerHost, player: u128, line: String) {
    if let Some(player) = host.player(player) {
        let _ = player.tell(host, Text::literal(line));
    }
}

/// A registration the game refused is a mod bug, not a runtime condition.
pub(crate) fn warn_refused(error: impl std::fmt::Display) {
    eprintln!(
        "Integrated Dynamics: the cable block entity part was refused: {error} -- the cable will not carry a channel"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cable_state_string_gives_its_six_parts() {
        let text = "integrateddynamics:cable[part_down=none,part_east=writer,part_north=none,part_south=none,part_up=none,part_west=reader,status=active]";
        let parts = parts_from_state_string(text).expect("a cable state parses");
        assert_eq!(parts.get(0), Some(Part::None));
        assert_eq!(parts.get(1), Some(Part::None));
        assert_eq!(parts.get(2), Some(Part::None));
        assert_eq!(parts.get(3), Some(Part::None));
        assert_eq!(parts.get(4), Some(Part::Reader));
        assert_eq!(parts.get(5), Some(Part::Writer));
    }

    #[test]
    fn another_blocks_state_string_is_not_a_cable() {
        assert_eq!(
            parts_from_state_string(
                "minecraft:oak_stairs[facing=east,half=bottom,shape=straight,waterlogged=false]"
            ),
            None
        );
    }

    #[test]
    fn a_cable_state_missing_a_face_is_refused() {
        let text = "integrateddynamics:cable[part_down=none,part_east=writer,part_north=none,part_south=none,part_up=none]";
        assert_eq!(parts_from_state_string(text), None);
    }

    #[test]
    fn an_unknown_part_value_is_refused() {
        let text = "integrateddynamics:cable[part_down=none,part_east=reader,part_north=none,part_south=none,part_up=sensor,part_west=none,status=active]";
        assert_eq!(parts_from_state_string(text), None);
    }

    #[test]
    fn a_state_without_properties_is_not_a_cable() {
        assert_eq!(parts_from_state_string("integrateddynamics:cable"), None);
    }

    #[test]
    fn the_face_property_names_the_engine_faces() {
        assert_eq!(face_property(0), "part_down");
        assert_eq!(face_property(1), "part_up");
        assert_eq!(face_property(2), "part_north");
        assert_eq!(face_property(3), "part_south");
        assert_eq!(face_property(4), "part_west");
        assert_eq!(face_property(5), "part_east");
    }
}
