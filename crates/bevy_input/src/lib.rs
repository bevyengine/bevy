mod input;
pub mod keyboard;
pub mod mouse;
pub mod system;

pub use input::*;

pub mod prelude {
    pub use crate::{keyboard::KeyCode, mouse::MouseButton, Input};
}

use bevy_app::prelude::*;
use keyboard::{keyboard_input_system, KeyCode, KeyboardInput};
use mouse::{
    mouse_button_input_system, MouseButton, MouseButtonInput, MouseMotion,
};

use bevy_ecs::IntoQuerySystem;

/// Adds keyboard and mouse input to an App
#[derive(Default)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<KeyboardInput>()
            .add_event::<MouseButtonInput>()
            .add_event::<MouseMotion>()
            .init_resource::<Input<KeyCode>>()
            .add_system_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                keyboard_input_system.system(),
            )
            .init_resource::<Input<MouseButton>>()
            .add_system_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                mouse_button_input_system.system(),
            );
    }
}
