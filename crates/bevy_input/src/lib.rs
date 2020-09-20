mod axis;
pub mod gamepad;
mod input;
pub mod keyboard;
pub mod mouse;
pub mod system;

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

use bevy_ecs::IntoQuerySystem;
use gamepad::{GamepadAxis, GamepadButton, GamepadEvent};

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
            .init_resource::<Input<GamepadButton>>()
            .init_resource::<Axis<GamepadAxis>>();
    }
}
