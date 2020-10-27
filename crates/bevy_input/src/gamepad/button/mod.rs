//! Gamepad input button module

pub mod axis_settings;
pub mod button_device;
pub mod button_settings;
pub mod button_type;

// Export public exports
pub use prelude::*;

pub mod prelude {
    pub use super::{
        axis_settings::ButtonAxisSettings, button_device::GamepadButton,
        button_settings::ButtonSettings, button_type::GamepadButtonType,
    };
}
