pub mod input;
pub mod keycode;
pub mod plugin;
pub mod system;

// Export public exports module
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use super::{
        input::{KeyboardInput, KeyboardInputState},
        keycode::KeyCode,
        plugin::KeyboardInputPlugin,
        system::keyboard_input_system,
    };
}
