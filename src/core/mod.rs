pub mod bytes;
pub mod event;
mod time;

pub use bytes::*;
pub use event::*;
pub use time::*;

use crate::app::{plugin::AppPlugin, AppBuilder};
use bevy_transform::transform_system_bundle;

#[derive(Default)]
pub struct CorePlugin;

impl AppPlugin for CorePlugin {
    fn build(&self, mut app: AppBuilder) -> AppBuilder {
        for transform_system in transform_system_bundle::build(&mut app.world).drain(..) {
            app = app.add_system(transform_system);
        }

        app.add_resource(Time::new())
    }
}