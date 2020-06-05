mod input;
pub mod keyboard;
pub mod mouse;
pub mod system;

pub use input::*;

use bevy_app::{AppBuilder, AppPlugin};
use keyboard::KeyboardInput;
use mouse::{MouseButtonInput, MouseMotionInput};
use legion::prelude::IntoSystem;

#[derive(Default)]
pub struct InputPlugin;

impl AppPlugin for InputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<KeyboardInput>()
            .add_event::<MouseButtonInput>()
            .add_event::<MouseMotionInput>()
            .init_resource::<Input>()
            .init_resource::<InputState>()
            .add_system_to_stage(bevy_app::stage::EVENT_UPDATE, input_system.system());
    }
}
