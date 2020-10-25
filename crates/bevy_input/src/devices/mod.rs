//! Bevy input device module
//!
//! Contains the code for the input devices that receive input updates

pub mod gamepad;

pub mod touch;

pub use gamepad::{Gamepad, GamepadAxis, GamepadButton};
pub use touch::{Touch, Touches};

// TODO look into naming device_resources
