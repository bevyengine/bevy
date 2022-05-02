use crate::input::Input;
use crate::keyboard::KeyCode;
use crate::{ButtonState, KeyCode, KeyboardInput};
use bevy_app::AppExit;
use bevy_ecs::prelude::{EventWriter, Res};

/// Sends an [`AppExit`] event whenever the `ESC` key is pressed.
///
/// ## Note
///
/// This system is not added as part of the `DefaultPlugins`. You can add the [`exit_on_esc_system`]
/// yourself if desired.
pub fn exit_on_esc_system(
    keyboard_input: Res<Input<KeyCode>>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    if keyboard_input.just_pressed(KeyCode::Escape) {
        app_exit_events.send_default();
   }
}
