//! Mouse input module

pub mod button;
pub mod motion;
pub mod plugin;
pub mod system;
pub mod wheel;

// Export public exports
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use super::{
        button::{MouseButton, MouseButtonInput, MouseButtonInputState},
        motion::MouseMotion,
        plugin::MouseInputPlugin,
        system::mouse_button_input_system,
        wheel::{MouseScrollUnit, MouseWheel},
    };
}
