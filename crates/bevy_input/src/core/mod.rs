//! Core input functionality module

pub mod axis;
pub mod element_state;
pub mod input;
pub mod input_plugin;
pub mod system;

// export public exports
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use super::{
        axis::Axis,
        element_state::ElementState,
        input::Input,
        input_plugin::InputPlugin,
        system::{exit_on_esc_system, ExitOnEscapeState},
    };
}
