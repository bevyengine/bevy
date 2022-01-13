/// The gamepad axis functionality.
pub mod axis;

/// The gamepad button functionality.
pub mod button;

/// The gamepad input events.
pub mod event;

/// The gamepad input resources.
pub mod gamepads;

/// The gamepad input settings.
pub mod settings;

/// The gamepad input systems.
pub mod system;

/// The gamepad input types.
pub mod types;

pub use crate::gamepad::{
    axis::*, button::*, event::*, gamepads::*, settings::*, system::*, types::*,
};
