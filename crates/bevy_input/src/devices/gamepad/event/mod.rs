//! Gamepad events module

pub mod event_type;
pub mod event_system;
pub mod gamepad_event;

// Export public exports
pub use prelude::*;

pub mod prelude {
    pub use super::{event_type::GamepadEventType, event_system::gamepad_event_system, gamepad_event::{GamepadEvent, GamepadEventRaw}};
}