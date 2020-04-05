pub mod keyboard;
pub mod mouse;

use crate::{app::AppBuilder, prelude::AppPlugin};
use keyboard::KeyboardInput;
use mouse::MouseInput;

#[derive(Default)]
pub struct InputPlugin;

impl AppPlugin for InputPlugin {
    fn build(&self, app: AppBuilder) -> AppBuilder {
        app.add_event::<KeyboardInput>()
            .add_event::<MouseInput>()
    }

    fn name(&self) -> &str {
        "Input"
    }
}
