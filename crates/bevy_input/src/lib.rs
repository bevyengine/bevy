#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevy.org/assets/icon.png",
    html_favicon_url = "https://bevy.org/assets/icon.png"
)]
#![no_std]

//! Input functionality for the [Bevy game engine](https://bevy.org/).
//!
//! # Supported input devices
//!
//! `bevy` currently supports keyboard, mouse, gamepad, and touch inputs.

#[cfg(feature = "std")]
extern crate std;

extern crate alloc;

mod axis;
mod button_input;
/// Common run conditions
pub mod common_conditions;
pub mod gamepad;
pub mod gestures;
pub mod keyboard;
pub mod mouse;
pub mod touch;

pub use axis::*;
pub use button_input::*;

/// The input prelude.
///
/// This includes the most common types in this crate, re-exported for your convenience.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        gamepad::{Gamepad, GamepadAxis, GamepadButton, GamepadSettings},
        keyboard::KeyCode,
        mouse::MouseButton,
        touch::{TouchInput, Touches},
        Axis, ButtonInput,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
#[cfg(feature = "bevy_reflect")]
use bevy_reflect::Reflect;
use gestures::*;
use keyboard::{keyboard_input_system, Key, KeyCode, KeyboardFocusLost, KeyboardInput};
use mouse::{
    accumulate_mouse_motion_system, accumulate_mouse_scroll_system, mouse_button_input_system,
    AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton, MouseButtonInput, MouseMotion,
    MouseWheel,
};
use touch::{touch_screen_input_system, TouchInput, Touches};

use gamepad::{
    gamepad_connection_system, gamepad_event_processing_system, GamepadAxisChangedEvent,
    GamepadButtonChangedEvent, GamepadButtonStateChangedEvent, GamepadConnectionEvent,
    GamepadEvent, GamepadRumbleRequest, RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent,
    RawGamepadEvent,
};

#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Adds keyboard and mouse input to an App
#[derive(Default)]
pub struct InputPlugin;

/// Label for systems that update the input data.
#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
pub struct InputSystems;

/// Deprecated alias for [`InputSystems`].
#[deprecated(since = "0.17.0", note = "Renamed to `InputSystems`.")]
pub type InputSystem = InputSystems;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app
            // keyboard
            .add_message::<KeyboardInput>()
            .add_message::<KeyboardFocusLost>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<Key>>()
            .add_systems(PreUpdate, keyboard_input_system.in_set(InputSystems))
            // mouse
            .add_message::<MouseButtonInput>()
            .add_message::<MouseMotion>()
            .add_message::<MouseWheel>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(
                PreUpdate,
                (
                    mouse_button_input_system,
                    accumulate_mouse_motion_system,
                    accumulate_mouse_scroll_system,
                )
                    .in_set(InputSystems),
            )
            .add_message::<PinchGesture>()
            .add_message::<RotationGesture>()
            .add_message::<DoubleTapGesture>()
            .add_message::<PanGesture>()
            // gamepad
            .add_message::<GamepadEvent>()
            .add_message::<GamepadConnectionEvent>()
            .add_message::<GamepadButtonChangedEvent>()
            .add_message::<GamepadButtonStateChangedEvent>()
            .add_message::<GamepadAxisChangedEvent>()
            .add_message::<RawGamepadEvent>()
            .add_message::<RawGamepadAxisChangedEvent>()
            .add_message::<RawGamepadButtonChangedEvent>()
            .add_message::<GamepadRumbleRequest>()
            .init_resource::<AccumulatedMouseMotion>()
            .init_resource::<AccumulatedMouseScroll>()
            .add_systems(
                PreUpdate,
                (
                    gamepad_connection_system,
                    gamepad_event_processing_system.after(gamepad_connection_system),
                )
                    .in_set(InputSystems),
            )
            // touch
            .add_message::<TouchInput>()
            .init_resource::<Touches>()
            .add_systems(PreUpdate, touch_screen_input_system.in_set(InputSystems));
    }
}

/// The current "press" state of an element
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq, Clone)
)]
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(
    all(feature = "serialize", feature = "bevy_reflect"),
    reflect(Serialize, Deserialize)
)]
pub enum ButtonState {
    /// The button is pressed.
    Pressed,
    /// The button is not pressed.
    Released,
}

impl ButtonState {
    /// Is this button pressed?
    pub fn is_pressed(&self) -> bool {
        matches!(self, ButtonState::Pressed)
    }
}
