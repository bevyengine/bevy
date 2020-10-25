use crate::{
    core::Button,
    device_codes::KeyCode,
    events::KeyboardEvent,
    state::{ElementState, KeyboardInputState},
};
use bevy_app::Events;
use bevy_ecs::{Local, Res, ResMut};

/// Updates the Input<KeyCode> resource with the latest KeyboardInput events
pub fn keyboard_input_system(
    mut state: Local<KeyboardInputState>,
    mut keyboard_input: ResMut<Button<KeyCode>>,
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
