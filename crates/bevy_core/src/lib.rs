pub mod bytes;
pub mod float_ord;
pub mod time;

use bevy_app::{stage, AppBuilder, AppPlugin};
use bevy_ecs::IntoQuerySystem;
use bevy_type_registry::RegisterType;
use time::{time_system, timer_system, Time, Timer};
use bevy_math::{Vec3, Vec2, Mat3, Mat4, Quat};

#[derive(Default)]
pub struct CorePlugin;

impl AppPlugin for CorePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Time>()
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
