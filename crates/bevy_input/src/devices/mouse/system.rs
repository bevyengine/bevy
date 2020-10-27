use crate::{
    core::{ElementState, Input},
    devices::mouse::{MouseButton, MouseButtonInput, MouseButtonInputState},
};
use bevy_app::prelude::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_math::Vec2;

/// Updates the Input<MouseButton> resource with the latest MouseButtonInput events
pub fn mouse_button_input_system(
    mut state: Local<MouseButtonInputState>,
    mut mouse_button_input: ResMut<Input<MouseButton>>,
    mouse_button_input_events: Res<Events<MouseButtonInput>>,
) {
    mouse_button_input.update();
    for event in state
        .mouse_button_input_event_reader
        .iter(&mouse_button_input_events)
    {
        match event.state {
            ElementState::Pressed => mouse_button_input.press(event.button),
            ElementState::Released => mouse_button_input.release(event.button),
        }
    }
}
