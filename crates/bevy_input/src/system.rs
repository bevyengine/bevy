use crate::{
    keyboard::{KeyCode, KeyboardInput},
    ElementState,
};
use bevy_app::{
    prelude::{EventReader, Events},
    AppExit,
};
use bevy_ecs::{Local, Res, ResMut};

/// Local "exit on escape" system state
#[derive(Default)]
pub struct ExitOnEscapeState {
    reader: EventReader<KeyboardInput>,
}

/// Sends the AppExit event whenever the "esc" key is pressed.
pub fn exit_on_esc_system(
    mut state: Local<ExitOnEscapeState>,
    keyboard_input_events: Res<Events<KeyboardInput>>,
    mut app_exit_events: ResMut<Events<AppExit>>,
) {
    for event in state.reader.iter(&keyboard_input_events) {
        if let Some(key_code) = event.key_code {
            if event.state == ElementState::Pressed && key_code == KeyCode::Escape {
                app_exit_events.send(AppExit);
            }
        }
    }
}
