//! Touch input module

pub mod plugin;
pub mod system;
pub mod touch;
pub mod touches;

// Export public exports
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use super::{plugin::TouchInputPlugin, system::{touch_screen_input_system, TouchSystemState}, touch::{Touch,TouchInput, TouchPhase}, touches::{Touches}};
}

