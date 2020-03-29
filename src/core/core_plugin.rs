use super::{Time, Window, plugin::AppPlugin};
use crate::{app::AppBuilder};

#[derive(Default)]
pub struct CorePlugin;

impl AppPlugin for CorePlugin {
    fn build(&self, app: AppBuilder) -> AppBuilder {
        app.add_resource(Window::default())
            .add_resource(Time::new())
    }

    fn name(&self) -> &'static str {
        "Core"
    }
}
