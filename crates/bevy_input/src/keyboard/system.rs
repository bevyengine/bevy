use crate::{
    keyboard::{KeyCode, KeyboardInput},
    ElementState, Input,
};
use bevy_ecs::{event::EventReader, system::ResMut};

/// Updates the `Input<KeyCode>` resource with the latest `KeyboardInput` events
pub fn keyboard_input_system(
    mut keyboard_input: ResMut<Input<KeyCode>>,
    mut keyboard_input_events: EventReader<KeyboardInput>,
) {
    keyboard_input.clear();
    for event in keyboard_input_events.iter() {
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
