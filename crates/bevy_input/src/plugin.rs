use crate::{
    gamepad::{
        gamepad_connection_system, gamepad_event_system, GamepadAxis, GamepadButton, GamepadEvent,
        GamepadEventRaw, GamepadSettings, Gamepads,
    },
    keyboard::{keyboard_input_system, KeyCode, KeyboardInput},
    mouse::{mouse_button_input_system, MouseButton, MouseButtonInput, MouseMotion, MouseWheel},
    touch::{touch_screen_input_system, TouchInput, Touches},
    Axis, Input,
};
use bevy_app::{App, CoreStage, Plugin};
use bevy_ecs::schedule::{ParallelSystemDescriptorCoercion, SystemLabel};

/// A [`Plugin`] that adds keyboard, mouse, gamepad, and touch input support to an [`App`].
///
/// ## Schedule
///
/// This plugin contains the following systems, which run during the [`CoreStage::PreUpdate`]:
/// - [`keyboard_input_system`] labeled [`InputSystem`]
/// - [`mouse_button_input_system`] labeled [`InputSystem`]
/// - [`gamepad_event_system`] labeled [`InputSystem`]
/// - [`touch_screen_input_system`] labeled [`InputSystem`]
/// - [`gamepad_connection_system`] runs after systems labeled [`InputSystem`]
#[derive(Default)]
pub struct InputPlugin;

/// A [`SystemLabel`] marking the input handling systems.
#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemLabel)]
pub struct InputSystem;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut App) {
        app
            // Keyboard
            .add_event::<KeyboardInput>()
            .init_resource::<Input<KeyCode>>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                keyboard_input_system.label(InputSystem),
            )
            // Mouse
            .add_event::<MouseButtonInput>()
            .add_event::<MouseMotion>()
            .add_event::<MouseWheel>()
            .init_resource::<Input<MouseButton>>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                mouse_button_input_system.label(InputSystem),
            )
            // Gamepad
            .add_event::<GamepadEvent>()
            .add_event::<GamepadEventRaw>()
            .init_resource::<GamepadSettings>()
            .init_resource::<Gamepads>()
            .init_resource::<Input<GamepadButton>>()
            .init_resource::<Axis<GamepadAxis>>()
            .init_resource::<Axis<GamepadButton>>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                gamepad_event_system.label(InputSystem),
            )
            .add_system_to_stage(
                CoreStage::PreUpdate,
                gamepad_connection_system.after(InputSystem),
            )
            // Touch
            .add_event::<TouchInput>()
            .init_resource::<Touches>()
            .add_system_to_stage(
                CoreStage::PreUpdate,
                touch_screen_input_system.label(InputSystem),
            );
    }
}
