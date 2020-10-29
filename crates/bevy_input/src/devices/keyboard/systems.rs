use super::*;
use crate::core::*;
use bevy_app::{
    prelude::{EventReader, Events},
    AppExit,
};
use bevy_ecs::{Local, Res, ResMut};

/// Local "exit on escape" system state
#[derive(Default)]
pub struct ExitOnEscapeState {
    reader: EventReader<KeyboardEvent>,
}

/// Sends the AppExit event whenever the "esc" key is pressed.
pub fn exit_on_esc_system(
    mut state: Local<ExitOnEscapeState>,
    keyboard_input_events: Res<Events<KeyboardEvent>>,
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

/// Updates the BinaryInput<KeyCode> resource with the latest KeyboardInput events
pub fn keyboard_input_system(
    mut state: Local<KeyboardInputState>,
    mut keyboard_input: ResMut<BinaryInput<KeyCode>>,
    keyboard_input_events: Res<Events<KeyboardEvent>>,
) {
    keyboard_input.update();
    for event in state
        .keyboard_input_event_reader
        .iter(&keyboard_input_events)
    {
        if let KeyboardEvent {
            key_code: Some(key_code),
            state,
            ..
        } = event
        {
            match state {
                ElementState::Pressed => keyboard_input.press(*key_code),
                ElementState::Released => keyboard_input.release(*key_code),
            }
        }
    }
}
