//! Input update system module

use crate::{
    core::{ElementState, Input},
    keyboard::{KeyCode, KeyboardInput, KeyboardInputState},
};
use bevy_app::prelude::*;
use bevy_ecs::{Local, Res, ResMut};

/// Updates the Input<KeyCode> resource with the latest KeyboardInput events
pub fn keyboard_input_system(
    mut state: Local<KeyboardInputState>,
    mut keyboard_input: ResMut<Input<KeyCode>>,
    keyboard_input_events: Res<Events<KeyboardInput>>,
) {
    keyboard_input.update();
    for event in state
        .keyboard_input_event_reader
        .iter(&keyboard_input_events)
    {
        if let KeyboardInput {
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
