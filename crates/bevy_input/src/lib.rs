mod input;
pub mod keyboard;
pub mod mouse;
pub mod system;

pub use input::*;

use bevy_app::prelude::*;
use keyboard::{keyboard_input_system, KeyCode, KeyboardInput, KeyboardInputState};
use mouse::{
    mouse_button_input_system, MouseButton, MouseButtonInput, MouseButtonInputState, MouseMotion,
};

use bevy_ecs::IntoQuerySystem;

#[derive(Default)]
pub struct InputPlugin;

impl AppPlugin for InputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<KeyboardInput>()
            .add_event::<MouseButtonInput>()
            .add_event::<MouseMotion>()
            .init_resource::<Input<KeyCode>>()
            .init_resource::<KeyboardInputState>()
            .add_system_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                keyboard_input_system.system(),
            )
            .init_resource::<Input<MouseButton>>()
            .init_resource::<MouseButtonInputState>()
            .add_system_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                mouse_button_input_system.system(),
            );
    }
}
