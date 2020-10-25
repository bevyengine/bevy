pub mod core;
pub mod device_codes;
pub mod devices;
pub mod events;
pub mod settings;
pub mod state;
pub mod systems;

pub use crate::core::{axis::*, button::*};

pub mod prelude {
    pub use crate::{
        core::plugins::*,
        device_codes::{GamepadAxisCode, GamepadButtonCode, KeyCode, MouseButtonCode},
        devices::{Gamepad, GamepadAxis, GamepadButton},
        events::{GamepadEvent, GamepadEventType},
        Axis, Button,
    };
}
