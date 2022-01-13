//! This crate contains the input functionality of the `Bevy` game engine.
//!
//! ## Supported input devices
//!
//! `Bevy` currently supports keyboard, mouse, gamepad, and touch inputs.
//!
//! ## How to use the input functionality
//!
//! To use the input functionality provided by `Bevy` you can add the [`InputPlugin`](crate::InputPlugin)
//! to your [`App`](bevy_app::App) using the `add_plugin` function. This plugin is also bundled into
//! the `DefaultPlugins` bundle, which can be added to your [`App`](bevy_app::App) using the `add_plugins`
//! function.

#[warn(missing_docs)]

/// The generic axis type.
mod axis;

/// The gamepad input functionality.
pub mod gamepad;

/// The generic input type.
mod input;

/// The keyboard input functionality.
pub mod keyboard;

/// The mouse input functionality.
pub mod mouse;

/// The app exiting functionality.
pub mod system;

/// The touch input functionality.
pub mod touch;

/// The input plugin.
pub mod plugin;

/// The input state.
pub mod state;

/// The `bevy_input` prelude.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        gamepad::{
            Gamepad, GamepadAxis, GamepadAxisType, GamepadButton, GamepadButtonType, GamepadEvent,
            GamepadEventType, Gamepads,
        },
        keyboard::KeyCode,
        mouse::MouseButton,
        touch::{TouchInput, Touches},
        Axis, Input,
    };
}

pub use axis::*;
pub use input::*;
pub use plugin::*;
pub use state::*;
