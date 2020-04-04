mod anchors;
mod margins;
mod node;
mod ui_update_system;

pub use anchors::*;
pub use margins::*;
pub use node::*;
pub use ui_update_system::*;

use crate::{app::AppBuilder, prelude::AppPlugin};

#[derive(Default)]
pub struct UiPlugin;

impl AppPlugin for UiPlugin {
    fn build(&self, app: AppBuilder) -> AppBuilder {
        app.add_system(ui_update_system())
    }
    fn name(&self) -> &str {
        "UI"
    }
}
