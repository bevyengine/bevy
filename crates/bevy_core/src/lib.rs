pub mod bytes;
pub mod time;
pub mod transform;

use bevy_app::{stage, AppBuilder, AppPlugin};
use bevy_component_registry::RegisterComponent;
use bevy_transform::{transform_system_bundle, components::{Children, LocalToParent, LocalToWorld, Translation, Rotation, Scale, NonUniformScale}};
use legion::prelude::IntoSystem;
use time::{start_timer_system, stop_timer_system, Time};

#[derive(Default)]
pub struct CorePlugin;

impl AppPlugin for CorePlugin {
    fn build(&self, app: &mut AppBuilder) {
        for transform_system in transform_system_bundle::build(app.world_mut()).drain(..) {
            app.add_system(transform_system);
        }

        app.init_resource::<Time>()
            .register_component::<Children>()
            .register_component::<LocalToParent>()
            .register_component::<LocalToWorld>()
            .register_component::<Translation>()
            .register_component::<Rotation>()
            .register_component::<Scale>()
            .register_component::<NonUniformScale>()
            .add_system_to_stage(stage::FIRST, start_timer_system.system())
            .add_system_to_stage(stage::LAST, stop_timer_system.system());
    }
}
