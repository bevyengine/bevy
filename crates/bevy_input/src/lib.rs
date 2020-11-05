pub mod core;
pub mod devices;

pub use crate::core::{axis::*, binary_input::*, plugins::*};

pub mod prelude {
    pub use crate::{core::*, devices::*};
}

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
