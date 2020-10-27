pub mod gamepad;
pub mod gamepad_plugin;
pub mod gamepad_settings;

// Export public exports
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use super::{
        gamepad::Gamepad, gamepad_plugin::GamepadInputPlugin, gamepad_settings::GamepadSettings,
    };
}
