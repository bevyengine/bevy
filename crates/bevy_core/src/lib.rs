mod bytes;
mod float_ord;
mod label;
mod time;

pub use bevy_app as app;
pub use bevy_ecs as ecs;
pub use bytes::*;
pub use float_ord::*;
pub use label::*;
pub use time::*;

pub mod prelude {
    pub use crate::{app::prelude::*, ecs::prelude::*, EntityLabels, Labels, Time, Timer};
}

use crate::prelude::*;
use bevy_math::{Mat3, Mat4, Quat, Vec2, Vec3};
use bevy_type_registry::RegisterType;

/// Adds core functionality to Apps.
#[derive(Default)]
pub struct CorePlugin;

impl Plugin for CorePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<Time>()
            .init_resource::<EntityLabels>()
            .register_component::<Timer>()
            .register_property::<Vec2>()
            .register_property::<Vec3>()
            .register_property::<Mat3>()
            .register_property::<Mat4>()
            .register_property::<Quat>()
            .register_property::<Option<String>>()
            .add_system_to_stage(stage::FIRST, time_system.system())
            .add_system_to_stage(stage::FIRST, timer_system.system())
            .add_system_to_stage(stage::PRE_UPDATE, entity_labels_system.system());
    }
}
