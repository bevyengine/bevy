#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Input functionality for the [Bevy game engine](https://bevyengine.org/).
//!
//! # Supported input devices
//!
//! `bevy` currently supports keyboard, mouse, gamepad, and touch inputs.

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
use keyboard::{keyboard_input_system, KeyCode, KeyboardFocusLost, KeyboardInput};
use mouse::{
    accumulate_mouse_motion_system, accumulate_mouse_scroll_system, mouse_button_input_system,
    AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton, MouseButtonInput, MouseMotion,
    MouseWheel,
};
use touch::{touch_screen_input_system, TouchInput, Touches};

use gamepad::{
    gamepad_connection_system, gamepad_event_processing_system, GamepadAxisChangedEvent,
    GamepadButtonChangedEvent, GamepadButtonStateChangedEvent, GamepadConnection,
    GamepadConnectionEvent, GamepadEvent, GamepadInfo, GamepadRumbleRequest, GamepadSettings,
    RawGamepadAxisChangedEvent, RawGamepadButtonChangedEvent, RawGamepadEvent,
};

#[cfg(all(feature = "serialize", feature = "bevy_reflect"))]
use bevy_reflect::{ReflectDeserialize, ReflectSerialize};

/// Adds keyboard and mouse input to an App
#[derive(Default)]
pub struct InputPlugin;

/// Label for systems that update the input data.
#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
pub struct InputSystem;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app
            // keyboard
            .add_event::<KeyboardInput>()
            .add_event::<KeyboardFocusLost>()
            .init_resource::<ButtonInput<KeyCode>>()
            .add_systems(PreUpdate, keyboard_input_system.in_set(InputSystem))
            // mouse
            .add_event::<MouseButtonInput>()
            .add_event::<MouseMotion>()
            .add_event::<MouseWheel>()
            .init_resource::<ButtonInput<MouseButton>>()
            .add_systems(
                PreUpdate,
                (
                    mouse_button_input_system,
                    accumulate_mouse_motion_system,
                    accumulate_mouse_scroll_system,
                )
                    .in_set(InputSystem),
            )
            .add_event::<PinchGesture>()
            .add_event::<RotationGesture>()
            .add_event::<DoubleTapGesture>()
            .add_event::<PanGesture>()
            // gamepad
            .add_event::<GamepadEvent>()
            .add_event::<GamepadConnectionEvent>()
            .add_event::<GamepadButtonChangedEvent>()
            .add_event::<GamepadButtonStateChangedEvent>()
            .add_event::<GamepadAxisChangedEvent>()
            .add_event::<RawGamepadEvent>()
            .add_event::<RawGamepadAxisChangedEvent>()
            .add_event::<RawGamepadButtonChangedEvent>()
            .add_event::<GamepadRumbleRequest>()
            .init_resource::<AccumulatedMouseMotion>()
            .init_resource::<AccumulatedMouseScroll>()
            .add_systems(
                PreUpdate,
                (
                    gamepad_connection_system,
                    gamepad_event_processing_system.after(gamepad_connection_system),
                )
                    .in_set(InputSystem),
            )
            // touch
            .add_event::<TouchInput>()
            .init_resource::<Touches>()
            .add_systems(PreUpdate, touch_screen_input_system.in_set(InputSystem));

        #[cfg(feature = "bevy_reflect")]
        {
            // Register common types
            app.register_type::<ButtonState>()
                .register_type::<KeyboardInput>()
                .register_type::<MouseButtonInput>()
                .register_type::<PinchGesture>()
                .register_type::<RotationGesture>()
                .register_type::<DoubleTapGesture>()
                .register_type::<PanGesture>()
                .register_type::<TouchInput>()
                .register_type::<RawGamepadEvent>()
                .register_type::<RawGamepadAxisChangedEvent>()
                .register_type::<RawGamepadButtonChangedEvent>()
                .register_type::<GamepadConnectionEvent>()
                .register_type::<GamepadButtonChangedEvent>()
                .register_type::<GamepadAxisChangedEvent>()
                .register_type::<GamepadButtonStateChangedEvent>()
                .register_type::<GamepadInfo>()
                .register_type::<GamepadConnection>()
                .register_type::<GamepadSettings>()
                .register_type::<AccumulatedMouseMotion>()
                .register_type::<AccumulatedMouseScroll>();
        }
    }
}

/// The current "press" state of an element
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
#[cfg_attr(
    feature = "bevy_reflect",
    derive(Reflect),
    reflect(Debug, Hash, PartialEq)
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
