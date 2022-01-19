/// The current state of a button.
///
/// ## Usage
///
/// It is used to define the state of the [`KeyboardInput`](crate::keyboard::KeyboardInput) and
/// [`MouseButtonInput`](crate::mouse::MouseButtonInput).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum ButtonState {
    /// Represents a pressed button.
    Pressed,
    /// Represents a released button.
    Released,
}

impl ButtonState {
    /// Returns `true` if the button is pressed.
    pub fn is_pressed(&self) -> bool {
        matches!(self, ButtonState::Pressed)
    }
}

#[cfg(test)]
mod tests {
    use crate::ButtonState;

    #[test]
    fn test_is_pressed() {
        assert!(ButtonState::Pressed.is_pressed());
        assert!(!ButtonState::Released.is_pressed());
    }
}
