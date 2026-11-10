// `is_signal_source`: a cable with a writer is a redstone signal source
// (`native_is_signal_source`, API 38).
fn observe_is_signal_source(
    level: &crate::game::GameLevel<'_>,
    state: BlockStateId,
    source: &mut bool,
) {
    crate::game::integrateddynamics_state::on_is_signal_source(level, state, source);
}
