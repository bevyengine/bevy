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
use gamepad::{gamepad_event_system, GamepadAxis, GamepadButton, GamepadEvent, GamepadSetting};

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
            .add_system_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                keyboard_input_system.system(),
            )
            .init_resource::<Input<MouseButton>>()
            .add_system_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                mouse_button_input_system.system(),
            )
            .add_event::<GamepadEvent>()
            .init_resource::<GamepadSetting>()
            .init_resource::<Input<GamepadButton>>()
            .init_resource::<Axis<GamepadAxis>>()
            .init_resource::<Axis<GamepadButton>>()
            .add_startup_system_to_stage(
                bevy_app::startup_stage::POST_STARTUP,
                gamepad_event_system.system(),
            )
            .add_system_to_stage(bevy_app::stage::EVENT_UPDATE, gamepad_event_system.system())
            .add_event::<TouchInput>()
            .init_resource::<Touches>()
            .add_system_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                touch_screen_input_system.system(),
            );
    }
}
