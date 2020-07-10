use super::keyboard::ElementState;
use crate::Input;
use bevy_app::{EventReader, Events};
use bevy_ecs::{Res, ResMut};
use glam::Vec2;

#[derive(Debug, Clone)]
pub struct MouseButtonInput {
    pub button: MouseButton,
    pub state: ElementState,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

#[derive(Debug, Clone)]
pub struct MouseMotion {
    pub delta: Vec2,
}

#[derive(Default)]
pub struct MouseButtonInputState {
    mouse_button_input_event_reader: EventReader<MouseButtonInput>,
}

pub fn mouse_button_input_system(
    mut state: ResMut<MouseButtonInputState>,
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
