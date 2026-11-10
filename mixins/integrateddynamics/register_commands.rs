// `/integrateddynamics info|part` is registered at the command hook's head
// (`native_register_commands`, API 8).
fn observe_register_commands(commands: &mut Vec<NativeCommand>) {
    crate::game::integrateddynamics_state::on_register_commands(commands);
}
