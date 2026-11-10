// After the level's tick: recompute every queued cable network
// (`native_level_tick_post`, API 23).
fn observe_level_tick_post(game: &mut crate::game::Game<'_>, dimension: &str) {
    crate::game::integrateddynamics_state::on_level_tick_post(game, dimension);
}
