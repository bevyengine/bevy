use crate::{
    device_codes::{MouseButtonCode, MouseScrollUnitCode},
    state::ElementState,
};
use bevy_math::Vec2;

/// A mouse button input event
#[derive(Debug, Clone)]
pub struct MouseButtonEvent {
    pub button: MouseButtonCode,
    pub state: ElementState,
}

/// A mouse motion event
#[derive(Debug, Clone)]
pub struct MouseMotionEvent {
    pub delta: Vec2,
}

/// A mouse scroll wheel event, where x represents horizontal scroll and y represents vertical scroll.
#[derive(Debug, Clone)]
pub struct MouseWheelEvent {
    pub unit: MouseScrollUnitCode,
    pub x: f32,
    pub y: f32,
}
