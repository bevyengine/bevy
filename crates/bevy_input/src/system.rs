use crate::{
    keyboard::{KeyCode, KeyboardInput},
    ElementState,
};
use bevy_app::AppExit;
use bevy_ecs::prelude::{EventReader, EventWriter};

/// Sends the [`AppExit`] event whenever the `ESC` key is pressed.
///
/// ## Note
///
/// This event is not used inside of `Bevy` by default. You can add the `exit_on_esc_system`
/// to your [`App`](bevy_app::App) by using the `add_system` function.
pub fn exit_on_esc_system(
    mut keyboard_input_events: EventReader<KeyboardInput>,
    mut app_exit_events: EventWriter<AppExit>,
) {
    for event in keyboard_input_events.iter() {
        if let Some(key_code) = event.key_code {
            if event.state == ElementState::Pressed && key_code == KeyCode::Escape {
                app_exit_events.send(AppExit);
            }
        }
    }
}
