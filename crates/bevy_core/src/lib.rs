mod bytes;
mod float_ord;
mod time;

pub use bytes::*;
pub use float_ord::*;
pub use time::*;

pub mod prelude {
    pub use crate::{Time, Timer};
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_math::{Mat3, Mat4, Quat, Vec2, Vec3};
use bevy_type_registry::RegisterType;

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
