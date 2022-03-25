use crate::ElementState;

/// A button on a mouse device
#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum MouseButton {
    Left,
    Right,
    Middle,
    Other(u16),
}

/// A mouse button input event
#[derive(Debug, Clone)]
pub struct MouseButtonInput {
    pub button: MouseButton,
    pub state: ElementState,
}
