//! Input configuration module
//!
//! Contains the code used to configure the behavior of device inputs.

pub mod axis_settings;
pub mod button_axis_settings;
pub mod button_settings;

pub mod gamepad;

pub use axis_settings::AxisSettings;
pub use button_axis_settings::ButtonAxisSettings;
pub use button_settings::ButtonSettings;
pub use gamepad::GamepadSettings;
