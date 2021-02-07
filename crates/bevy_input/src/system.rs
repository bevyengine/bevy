use crate::{
    keyboard::{KeyCode, KeyboardInput},
    ElementState,
};
use bevy_app::{
    prelude::{EventReader, Events},
    AppExit,
};
use bevy_ecs::ResMut;

/// Sends the AppExit event whenever the "esc" key is pressed.
pub fn exit_on_esc_system(
    mut keyboard_input_events: EventReader<KeyboardInput>,
    mut app_exit_events: ResMut<Events<AppExit>>,
) {
    for event in keyboard_input_events.iter() {
        if let Some(key_code) = event.key_code {
            if event.state == ElementState::Pressed && key_code == KeyCode::Escape {
                app_exit_events.send(AppExit);
            }
        }
    }
}
