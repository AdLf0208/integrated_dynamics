// `block_signal`: a writer face answers the cable's channel value as a weak
// signal (`native_block_signal`, API 38; hot path, no writes).
fn observe_block_signal(
    level: &crate::game::GameLevel<'_>,
    pos: BlockPos,
    state: BlockStateId,
    face: Direction,
    direct: bool,
    signal: &mut i32,
) {
    crate::game::integrateddynamics_state::on_block_signal(
        level, pos, state, face, direct, signal,
    );
}
