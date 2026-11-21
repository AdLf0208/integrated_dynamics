//! Integrated Dynamics for Horizon: a Logic Cable network whose parts carry a
//! redstone channel.
//!
//! Upstream a Logic Cable is a multipart block entity: each of its six faces can
//! hold one part, and every part's aspect reads or writes a value on the network.
//! This slice keeps the six parts as the cable's `part_<face>` state properties
//! (so the model can show them and they persist without a block-entity write),
//! keeps the network channel value in the cable's saved compound, and after the
//! level's tick recomputes every connected network from the cables that ticked
//! this tick: the channel is the strongest reader input (a Redstone Reader's
//! signal or a World Reader's light/weather/day aspect), and every Redstone
//! Writer on the network emits it from its own face.
//!
//! The rules live in [`runtime`], which has no game access and is unit-tested on
//! its own. This file is the one `impl Hooks` plus the cable's part slots; the game
//! calls them only for the cable block, and each is a thin call into [`support`].
pub mod runtime;
mod support;

use std::sync::Arc;

use mod_api::{
    self, BlockChangedEvent, BlockEntityEvent, BlockEntityPart, BlockEntityTickEvent,
    BlockEntityTicksEvent, BlockPart, BlockSignalEvent, ChunkUnloadEvent, Flow, Hooks, Host,
    HostSide, IsSignalSourceEvent, LevelHost, LevelTickPostEvent, LevelUnloadEvent,
    RegisterCommandsEvent, Registrar, RegistrationContext, RunModCommandEvent, ServerHost,
    UseItemOnEvent,
};

/// The Logic Cable block (and its item).
pub(crate) const CABLE: &str = "integrateddynamics:cable";

/// The Redstone Reader part item.
pub(crate) const READER: &str = "integrateddynamics:part_redstone_reader";

/// The Redstone Writer part item.
pub(crate) const WRITER: &str = "integrateddynamics:part_redstone_writer";

/// The World Reader part item.
pub(crate) const WORLD_READER: &str = "integrateddynamics:part_world_reader";

/// The one Part attached to the Logic Cable: its block behaviours (the signal it
/// gives, whether it is a source, the part items clicked onto a face) and its
/// block entity (the saved channel value and the tick that queues the network).
///
/// A moment that belongs to one block is a Part, not a global hook, so the game
/// asks into this type only for states of `integrateddynamics:cable`.
struct Cable;

#[mod_api::horizon_part]
impl BlockPart for Cable {
    fn block_signal(&self, level: &dyn LevelHost, event: &mut BlockSignalEvent) {
        support::on_block_signal(level, event);
    }

    fn is_signal_source(&self, level: &dyn LevelHost, event: &mut IsSignalSourceEvent) {
        support::on_is_signal_source(level, event);
    }

    fn use_item_on(
        &self,
        host: &mut dyn ServerHost,
        event: &mut UseItemOnEvent<'_>,
    ) -> Option<mod_api::InteractionResult> {
        support::on_use_item_on(host, event)
    }
}

/// The cable's block-entity Part (the saved channel value and the tick that queues its network).
struct CableEntity;

#[mod_api::horizon_part]
impl BlockPart for CableEntity {}

#[mod_api::horizon_part]
impl BlockEntityPart for CableEntity {
    fn block_entity(
        &self,
        level: mod_api::LevelSide,
        event: &mut BlockEntityEvent<'_>,
    ) -> Option<mod_api::Compound> {
        support::on_block_entity(level, event)
    }

    fn block_entity_ticks(&self, _host: &dyn Host, event: &mut BlockEntityTicksEvent<'_>) {
        // Every state of the cable holds a channel and re-evaluates its network.
        event.ticks = true;
    }

    fn block_entity_tick(&self, host: &mut dyn ServerHost, event: &mut BlockEntityTickEvent<'_>) {
        support::on_block_entity_tick(host, event);
    }
}

/// The mod itself holds nothing: the cable's cross-call state is in `support`,
/// because a Part is a separate object the `Hooks` instance cannot reach.
#[derive(Default)]
struct Mod;

#[mod_api::horizon_mod]
impl Hooks for Mod {
    /// Attach the cable's Part to `integrateddynamics:cable` and declare the
    /// block entity type that Part owns.
    fn register(&self, r: &mut dyn Registrar, _cx: &RegistrationContext) {
        let cable = r.block_key(CABLE);
        r.attach_block_part(cable, "cable", Arc::new(Cable));
        let kind = r.declare_block_entity_type("cable", vec![CABLE.to_owned()]);
        // A separate type from the block Part: the engine refuses one part type attached to a
        // block id twice, and a block-entity part registers against its block.
        if let Err(error) = r.attach_block_entity_part(&kind, "cable_entity", Arc::new(CableEntity))
        {
            support::warn_refused(error);
        }
    }

    /// After a level's tick: recompute every cable network that ticked in this
    /// level, on the server.
    fn level_tick_post(&self, host: HostSide, event: &mut LevelTickPostEvent) {
        if let HostSide::Server(host) = host {
            support::on_level_tick_post(host, event.level);
        }
    }

    /// A cable was replaced: its cached channel value goes with it (the map used to keep every
    /// ticked position until the level unloaded).
    fn block_changed(&self, level: &dyn LevelHost, event: &mut BlockChangedEvent) {
        support::on_block_changed(level, event);
    }

    /// A chunk unloaded: cables in it are no longer in service, so their cached values go too.
    fn chunk_unload(&self, _host: &mut dyn ServerHost, event: &mut ChunkUnloadEvent<'_>) {
        support::on_chunk_unload(event.dimension, event.chunk);
    }

    /// A level closed: forget what was kept for it.
    fn level_unload(&self, _host: &mut dyn ServerHost, event: &mut LevelUnloadEvent<'_>) {
        support::on_level_unload(event.dimension);
    }

    /// `/integrateddynamics info|part|probe` (the harness readout and the path
    /// a click cannot reach).
    fn register_commands(&self, host: &mut dyn ServerHost, event: &mut RegisterCommandsEvent<'_>) {
        support::on_register_commands(host, event);
    }

    fn run_mod_command(
        &self,
        host: &mut dyn ServerHost,
        event: &mut RunModCommandEvent<'_>,
    ) -> Flow {
        support::on_run_mod_command(host, event)
    }
}
