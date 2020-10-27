//! Gamepad input axis module


pub mod settings;
pub mod axis_type;
pub mod gamepad_axis;

// export public exports
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use super::{axis_type::GamepadAxisType, settings::AxisSettings, gamepad_axis::GamepadAxis};
}