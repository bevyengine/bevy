pub mod core;
pub mod devices;

// Export public exports module
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use crate::{
        core::{Axis, Input, InputPlugin},
        devices::{
            gamepad::{
                Gamepad, GamepadAxis, GamepadAxisType, GamepadButton, GamepadButtonType,
                GamepadEvent, GamepadEventType,
            },
            keyboard::{KeyCode, KeyboardInputPlugin},
            mouse::{MouseButton, MouseInputPlugin},
            touch::{Touch, TouchInputPlugin},
        },
    };
}
