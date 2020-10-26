//! Gamepad input module

use crate::{Axis, Input};
use bevy_app::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_utils::HashMap;

pub mod gamepad_plugin;

// Export public exports module
pub use prelude::*;

/// Public exports module
pub mod prelude {
    pub use super::gamepad_plugin::GamepadInputPlugin;
}
