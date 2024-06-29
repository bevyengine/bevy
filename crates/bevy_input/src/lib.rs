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

/// Most commonly used re-exported types.
pub mod prelude {
    #[doc(hidden)]
    pub use crate::{
        gamepad::{
            Gamepad, GamepadAxis, GamepadAxisType, GamepadButton, GamepadButtonType, Gamepads,
        },
        keyboard::KeyCode,
        mouse::MouseButton,
        touch::{TouchInput, Touches},
        Axis, ButtonInput,
    };
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_reflect::Reflect;
use gestures::*;
use keyboard::{keyboard_input_system, KeyCode, KeyboardFocusLost, KeyboardInput};
use mouse::{mouse_button_input_system, MouseButton, MouseButtonInput, MouseMotion, MouseWheel};
use touch::{touch_screen_input_system, TouchInput, Touches};

use gamepad::{
    gamepad_axis_event_system, gamepad_button_event_system, gamepad_connection_system,
    gamepad_event_system, GamepadAxis, GamepadAxisChangedEvent, GamepadButton,
    GamepadButtonChangedEvent, GamepadButtonInput, GamepadConnectionEvent, GamepadEvent,
    GamepadRumbleRequest, GamepadSettings, Gamepads,
};

#[cfg(feature = "serialize")]
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
            .add_systems(PreUpdate, mouse_button_input_system.in_set(InputSystem))
            .add_event::<PinchGesture>()
            .add_event::<RotationGesture>()
            .add_event::<DoubleTapGesture>()
            .add_event::<PanGesture>()
            // gamepad
            .add_event::<GamepadConnectionEvent>()
            .add_event::<GamepadButtonChangedEvent>()
            .add_event::<GamepadButtonInput>()
            .add_event::<GamepadAxisChangedEvent>()
            .add_event::<GamepadEvent>()
            .add_event::<GamepadRumbleRequest>()
            .init_resource::<GamepadSettings>()
            .init_resource::<Gamepads>()
            .init_resource::<ButtonInput<GamepadButton>>()
            .init_resource::<Axis<GamepadAxis>>()
            .init_resource::<Axis<GamepadButton>>()
            .add_systems(
                PreUpdate,
                (
                    gamepad_event_system,
                    gamepad_connection_system.after(gamepad_event_system),
                    gamepad_button_event_system
                        .after(gamepad_event_system)
                        .after(gamepad_connection_system),
                    gamepad_axis_event_system
                        .after(gamepad_event_system)
                        .after(gamepad_connection_system),
                )
                    .in_set(InputSystem),
            )
            // touch
            .add_event::<TouchInput>()
            .init_resource::<Touches>()
            .add_systems(PreUpdate, touch_screen_input_system.in_set(InputSystem));

        // Register common types
        app.register_type::<ButtonState>()
            .register_type::<KeyboardInput>()
            .register_type::<MouseButtonInput>()
            .register_type::<PinchGesture>()
            .register_type::<RotationGesture>()
            .register_type::<DoubleTapGesture>()
            .register_type::<PanGesture>()
            .register_type::<TouchInput>()
            .register_type::<GamepadEvent>()
            .register_type::<GamepadButtonInput>()
            .register_type::<GamepadSettings>();
    }
}

/// The current "press" state of an element
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Reflect)]
#[reflect(Debug, Hash, PartialEq)]
#[cfg_attr(
    feature = "serialize",
    derive(serde::Serialize, serde::Deserialize),
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
