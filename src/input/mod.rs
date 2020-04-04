pub mod keyboard;

use crate::{app::AppBuilder, prelude::AppPlugin};
use keyboard::KeyboardInput;

#[derive(Default)]
pub struct InputPlugin;

impl AppPlugin for InputPlugin {
    fn build(&self, app: AppBuilder) -> AppBuilder {
        app.add_event::<KeyboardInput>()
    }

    fn name(&self) -> &str {
        "Input"
    }
}
