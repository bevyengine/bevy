use super::keyboard::ElementState;
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
pub struct MouseMotionInput {
    pub delta: Vec2,
}