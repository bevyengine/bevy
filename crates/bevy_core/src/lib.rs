pub mod bytes;
pub mod time;
pub mod transform;

use bevy_app::{stage, AppBuilder, AppPlugin};
use bevy_transform::transform_system_bundle;
use legion::prelude::IntoSystem;
use time::{start_timer_system, stop_timer_system};

#[derive(Default)]
pub struct CorePlugin;

impl AppPlugin for CorePlugin {
    fn build(&self, app: &mut AppBuilder) {
        for transform_system in transform_system_bundle::build(app.world_mut()).drain(..) {
            app.add_system(transform_system);
        }

        app.add_resource(time::Time::new())
            .add_system_to_stage(stage::FIRST, start_timer_system.system())
            .add_system_to_stage(stage::LAST, stop_timer_system.system());
    }
}
