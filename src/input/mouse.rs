use super::keyboard::ElementState;

#[derive(Debug, Clone)]
pub struct MouseInput {
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