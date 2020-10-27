use crate::core::{ElementState, Input};
use bevy_app::prelude::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_math::Vec2;
use bevy_input::core::element_state::ElementState;

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

/// State used by the mouse button input system
#[derive(Default)]
pub struct MouseButtonInputState {
    mouse_button_input_event_reader: EventReader<MouseButtonInput>,
}

