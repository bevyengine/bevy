//! bevy_input core module
//!
//! Contains code which is:
//!  - Fundamental to the functionality of the input plugin
//!  - Shared between all devices

pub(crate) mod axis;
pub(crate) mod binary_input;
pub mod plugins;
pub mod settings;
pub mod state;

pub use axis::Axis;
pub use binary_input::BinaryInput;
pub use plugins::{GamepadPlugin, KeyboardPlugin, MousePlugin, TouchPlugin};
pub use settings::*;
pub use state::ElementState;
