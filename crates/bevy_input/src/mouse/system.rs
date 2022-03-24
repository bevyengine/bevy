use crate::{
    mouse::{MouseButton, MouseButtonInput},
    ElementState, Input,
};
use bevy_ecs::{event::EventReader, system::ResMut};

/// Updates the `Input<MouseButton>` resource with the latest `MouseButtonInput` events
pub fn mouse_button_input_system(
    mut mouse_button_input: ResMut<Input<MouseButton>>,
    mut mouse_button_input_events: EventReader<MouseButtonInput>,
) {
    mouse_button_input.clear();
    for event in mouse_button_input_events.iter() {
        match event.state {
            ElementState::Pressed => mouse_button_input.press(event.button),
            ElementState::Released => mouse_button_input.release(event.button),
        }
    }
}
