//! bevy_input core module
//!
//! Contains code which is:
//!  - Fundamental to the functionality of the input plugin
//!  - Shared between all devices

pub(crate) mod axis;
pub(crate) mod button;
pub mod plugins;

pub use axis::Axis;
pub use button::Button;
pub use plugins::{GamepadPlugin, InputPlugin, KeyboardPlugin, MousePlugin, TouchPlugin};
