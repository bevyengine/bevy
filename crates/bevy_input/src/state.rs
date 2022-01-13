/// The current state of an element.
///
/// ## Usage
///
/// It is used to define the state of the [`KeyboardInput`](crate::keyboard::KeyboardInput) and
/// [`MouseButtonInput`](crate::mouse::MouseButtonInput).
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
pub enum ElementState {
    /// Represents a pressed element or key.
    Pressed,
    /// Represents a released element or key.
    Released,
}

impl ElementState {
    /// Returns `true` if the element is pressed.
    pub fn is_pressed(&self) -> bool {
        matches!(self, ElementState::Pressed)
    }
}

#[cfg(test)]
mod tests {
    use crate::ElementState;

    #[test]
    fn test_is_pressed() {
        assert!(ElementState::Pressed.is_pressed());
        assert!(!ElementState::Released.is_pressed());
    }
}
