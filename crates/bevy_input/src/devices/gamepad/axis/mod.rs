//! Gamepad input axis module

pub mod axis_type;
pub mod gamepad_axis;
pub mod settings;

// export public exports
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use super::{
        axis_type::GamepadAxisType, gamepad_axis::GamepadAxis, settings::AxisSettings,
    };
}
