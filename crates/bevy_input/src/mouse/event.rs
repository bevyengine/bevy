use crate::{
    mouse::{MouseButton, MouseScrollUnit},
    ElementState,
};
use bevy_math::Vec2;

/// A mouse button input event
#[derive(Debug, Clone)]
pub struct MouseButtonInput {
    pub button: MouseButton,
    pub state: ElementState,
}

/// A mouse motion event
#[derive(Debug, Clone)]
pub struct MouseMotion {
    pub delta: Vec2,
}

/// A mouse scroll wheel event, where x represents horizontal scroll and y represents vertical
/// scroll.
#[derive(Debug, Clone)]
pub struct MouseWheel {
    pub unit: MouseScrollUnit,
    pub x: f32,
    pub y: f32,
}
