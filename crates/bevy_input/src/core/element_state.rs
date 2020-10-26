/// The current "press" state of an element
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ElementState {
    Pressed,
    Released,
}

impl ElementState {
    pub fn is_pressed(&self) -> bool {
        matches!(self, ElementState::Pressed)
    }
}
