use crate::{ElementState, KeyCode, KeyboardInput};
use bevy_app::AppExit;
use bevy_ecs::prelude::{EventReader, EventWriter};

/// Sends an [`AppExit`] event whenever the `ESC` key is pressed.
///
/// ## Note
///
/// This system is not added as part of the `DefaultPlugins`. You can add the [`exit_on_esc_system`]
/// yourself if desired.
pub fn exit_on_esc_system(
    mut keyboard_input_events: EventReader<KeyboardInput>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    for event in keyboard_input_events.iter() {
        if let Some(key_code) = event.key_code {
            if event.state == ElementState::Pressed && key_code == KeyCode::Escape {
                app_exit_events.send_default();
            }
        }
    }
}
