mod axis;
pub mod gamepad;
mod input;
pub mod keyboard;
pub mod mouse;
pub mod system;
pub mod touch;

pub use axis::*;
use bevy_ecs::IntoSystem;
pub use input::*;

pub mod prelude {
    pub use crate::{
        gamepad::{
            Gamepad, GamepadAxis, GamepadAxisType, GamepadButton, GamepadButtonType, GamepadEvent,
            GamepadEventType,
        },
        keyboard::KeyCode,
        mouse::MouseButton,
        touch::{TouchInput, Touches},
        Axis, Input,
    };
}

use bevy_app::prelude::*;
use keyboard::{keyboard_input_system, KeyCode, KeyboardInput};
use mouse::{mouse_button_input_system, MouseButton, MouseButtonInput, MouseMotion, MouseWheel};
use touch::{touch_screen_input_system, TouchInput, Touches};

use gamepad::{
    gamepad_event_system, GamepadAxis, GamepadButton, GamepadEvent, GamepadEventRaw,
    GamepadSettings,
};

/// Adds keyboard and mouse input to an App
#[derive(Default)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<KeyboardInput>()
            .add_event::<MouseButtonInput>()
            .add_event::<MouseMotion>()
            .add_event::<MouseWheel>()
            .init_resource::<Input<KeyCode>>()
            .add_system_to_stage(bevy_app::stage::EVENT, keyboard_input_system.system())
            .init_resource::<Input<MouseButton>>()
            .add_system_to_stage(bevy_app::stage::EVENT, mouse_button_input_system.system())
            .add_event::<GamepadEvent>()
            .add_event::<GamepadEventRaw>()
            .init_resource::<GamepadSettings>()
            .init_resource::<Input<GamepadButton>>()
            .init_resource::<Axis<GamepadAxis>>()
            .init_resource::<Axis<GamepadButton>>()
            .add_system_to_stage(bevy_app::stage::EVENT, gamepad_event_system.system())
            .add_startup_system_to_stage(
                bevy_app::startup_stage::STARTUP,
                gamepad_event_system.system(),
            )
            .add_event::<TouchInput>()
            .init_resource::<Touches>()
            .add_system_to_stage(bevy_app::stage::EVENT, touch_screen_input_system.system());
    }
}

/// The current "press" state of an element
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum ElementState {
    Pressed,
    Released,
}

impl ElementState {
    pub fn is_pressed(&self) -> bool {
        matches!(self, ElementState::Pressed)
    }
}
