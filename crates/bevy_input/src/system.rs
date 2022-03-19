use crate::input::Input;
use crate::keyboard::KeyCode;
use bevy_app::AppExit;
use bevy_ecs::prelude::{EventWriter, Res};

/// Sends the `AppExit` event whenever the "esc" key is pressed.
pub fn exit_on_esc_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        app_exit_events.send_default();
    }
}
