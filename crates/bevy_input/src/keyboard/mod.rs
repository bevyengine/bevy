pub mod input;
pub mod input_system;
pub mod keyboard_plugin;
pub mod keycode;

// Export public exports module
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use super::{
        input::{KeyboardInput, KeyboardInputState},
        input_system::keyboard_input_system,
        keyboard_plugin::KeyboardInputPlugin,
        keycode::KeyCode,
    };
}
