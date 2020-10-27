pub mod input;
pub mod system;
pub mod plugin;
pub mod keycode;

// Export public exports module
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use super::{
        input::{KeyboardInput, KeyboardInputState},
        system::keyboard_input_system,
        plugin::KeyboardInputPlugin,
        keycode::KeyCode,
    };
}
