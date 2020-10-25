mod axis;
pub mod gamepad;
mod input;
pub mod keyboard;
pub mod mouse;
pub mod system;
pub mod touch;

pub use axis::*;
pub use input::*;

pub mod prelude {
    pub use crate::{
        gamepad::{
            Gamepad, GamepadAxis, GamepadAxisType, GamepadButton, GamepadButtonType, GamepadEvent,
            GamepadEventType,
        },
        keyboard::KeyCode,
        mouse::MouseButton,
        Axis, Input,
    };
}

use bevy_app::prelude::*;
use keyboard::{keyboard_input_system, KeyCode, KeyboardInput};
use mouse::{mouse_button_input_system, MouseButton, MouseButtonInput, MouseMotion, MouseWheel};
use touch::{touch_screen_input_system, TouchInput, Touches};

use bevy_ecs::IntoQuerySystem;
use gamepad::{
    gamepad_event_system, GamepadAxis, GamepadButton, GamepadEvent, GamepadEventRaw,
    GamepadSettings,
};

/// Adds keyboard, mouse, gamepad, and touch input to an App
#[derive(Default)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(KeyboardInputPlugin)
            .add_plugin(MouseInputPlugin)
            .add_plugin(GamepadInputPlugin)
            .add_plugin(TouchInputPlugin);
    }
}

/// Adds gamepad input to an App
#[derive(Default)]
pub struct GamepadInputPlugin;

impl Plugin for GamepadInputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<GamepadEvent>()
            .add_event::<GamepadEventRaw>()
            .init_resource::<GamepadSettings>()
            .init_resource::<Input<GamepadButton>>()
            .init_resource::<Axis<GamepadAxis>>()
            .init_resource::<Axis<GamepadButton>>()
            .add_system_to_stage(bevy_app::stage::EVENT, gamepad_event_system.system());
    }
}

/// Adds keyboard input to an App
#[derive(Default)]
pub struct KeyboardInputPlugin;

impl Plugin for KeyboardInputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<KeyboardInput>()
            .init_resource::<Input<KeyCode>>()
            .add_system_to_stage(bevy_app::stage::EVENT, keyboard_input_system.system());
    }
}

/// Adds mouse input to an App
#[derive(Default)]
pub struct MouseInputPlugin;

impl Plugin for MouseInputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<MouseButtonInput>()
            .add_event::<MouseMotion>()
            .add_event::<MouseWheel>()
            .init_resource::<Input<MouseButton>>()
            .add_system_to_stage(bevy_app::stage::EVENT, mouse_button_input_system.system());
    }
}

/// Adds keyboard, mouse, gamepad, and touch input to an App
#[derive(Default)]
pub struct TouchInputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<TouchInput>()
            .init_resource::<Touches>()
            .add_system_to_stage(bevy_app::stage::EVENT, touch_screen_input_system.system());
    }
}
