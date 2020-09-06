use super::keyboard::ElementState;
use crate::Input;
use bevy_app::prelude::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_math::Vec2;

/// A mouse button input event
#[derive(Debug, Clone)]
pub struct MouseButtonInput {
    pub button: MouseButton,
    pub state: ElementState,
}

/// A button on a mouse device
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u8),
}

/// A mouse motion event
#[derive(Debug, Clone)]
pub struct MouseMotion {
    pub delta: Vec2,
}

/// Unit of scroll
#[derive(Debug, Clone, Copy)]
pub enum MouseScrollUnit {
    Line,
    Pixel,
}

/// A mouse scroll wheel event, where x represents horizontal scroll and y represents vertical scroll.
#[derive(Debug, Clone)]
pub struct MouseWheel {
    pub unit: MouseScrollUnit,
    pub x: f32,
    pub y: f32,
}

/// State used by the mouse button input system
#[derive(Default)]
pub struct MouseButtonInputState {
    mouse_button_input_event_reader: EventReader<MouseButtonInput>,
}

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
