mod axis;
pub mod gamepad;
mod input;
pub mod keyboard;
pub mod mouse;
mod plugins;
pub mod system;
pub mod touch;

pub use axis::*;
pub use input::*;
pub use plugins::prelude::*;

pub mod prelude {
    pub use crate::{
        gamepad::{
            Gamepad, GamepadAxis, GamepadAxisType, GamepadButton, GamepadButtonType, GamepadEvent,
            GamepadEventType,
        },
        keyboard::KeyCode,
        mouse::MouseButton,
        plugins::prelude::*,
        Axis, Input,
    };
}
