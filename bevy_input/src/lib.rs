pub mod keyboard;
pub mod mouse;

use bevy_app::{AppBuilder, AppPlugin};
use keyboard::KeyboardInput;
use mouse::{MouseButtonInput, MouseMotion};

#[derive(Default)]
pub struct InputPlugin;

impl AppPlugin for InputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<KeyboardInput>()
            .add_event::<MouseButtonInput>()
            .add_event::<MouseMotion>();
    }
}
