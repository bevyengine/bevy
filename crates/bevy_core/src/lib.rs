pub mod bytes;
pub mod time;
pub mod transform;

use bevy_app::{stage, AppBuilder, AppPlugin};
use bevy_transform::{
    components::{
        Children, LocalToParent, LocalToWorld, NonUniformScale, Rotation, Scale, Translation,
    },
    transform_system_bundle,
};
use bevy_type_registry::RegisterType;
use glam::{Mat3, Mat4, Quat, Vec2, Vec3};
use legion::prelude::IntoSystem;
use time::{time_system, timer_system, Time, Timer};

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
            .register_component::<Timer>()
            .register_property_type::<Vec2>()
            .register_property_type::<Vec3>()
            .register_property_type::<Mat3>()
            .register_property_type::<Mat4>()
            .register_property_type::<Quat>()
            .add_system_to_stage(stage::FIRST, time_system.system())
            .add_system_to_stage(stage::FIRST, timer_system.system());
    }
}
