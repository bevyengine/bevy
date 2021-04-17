use crate::{ElementState, Input};
use bevy_ecs::{event::EventReader, system::ResMut};
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
    Other(u16),
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

/// A mouse scroll wheel event, where x represents horizontal scroll and y represents vertical
/// scroll.
#[derive(Debug, Clone)]
pub struct MouseWheel {
    pub unit: MouseScrollUnit,
    pub x: f32,
    pub y: f32,
}

/// Updates the Input<MouseButton> resource with the latest MouseButtonInput events
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
