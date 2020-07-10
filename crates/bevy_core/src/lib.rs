pub mod bytes;
pub mod float_ord;
pub mod time;
pub mod transform;

use bevy_app::{stage, startup_stage, AppBuilder, AppPlugin};
use bevy_transform::{
    components::{
        Children, LocalTransform, NonUniformScale, Rotation, Scale, Transform, Translation,
    },
    build_systems,
};
use bevy_type_registry::RegisterType;
use glam::{Mat3, Mat4, Quat, Vec2, Vec3};
use time::{time_system, timer_system, Time, Timer};
use bevy_ecs::IntoQuerySystem;

#[derive(Default)]
pub struct CorePlugin;

impl AppPlugin for CorePlugin {
    fn build(&self, app: &mut AppBuilder) {
        // we also add a copy of transform systems to startup to ensure we begin with correct transform/parent state
        for transform_system in build_systems() {
            app.add_startup_system_to_stage(startup_stage::POST_STARTUP, transform_system);
        }
        for transform_system in build_systems() {
            app.add_system_to_stage(stage::POST_UPDATE, transform_system);
        }

        app.init_resource::<Time>()
            .register_component::<Children>()
            .register_component::<LocalTransform>()
            .register_component::<Transform>()
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
