pub mod components;
pub mod hierarchy;
pub mod transform_propagate_system;

pub mod prelude {
    pub use crate::{components::*, hierarchy::*, TransformPlugin};
}

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use bevy_type_registry::RegisterType;
use prelude::{Children, NonUniformScale, Parent, Rotation, Scale, Transform, Translation};

pub(crate) fn transform_systems() -> Vec<Box<dyn System>> {
    let mut systems = Vec::with_capacity(5);

    systems.append(&mut hierarchy::hierarchy_maintenance_systems());
    systems.push(transform_propagate_system::transform_propagate_system.system());

    systems
}

#[derive(Default)]
pub struct TransformPlugin;

impl Plugin for TransformPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.register_component::<Children>()
            .register_component::<Parent>()
            .register_component::<Transform>()
            .register_component::<Translation>()
            .register_component::<Rotation>()
            .register_component::<Scale>()
            .register_component::<NonUniformScale>()
            // add transform systems to startup so the first update is "correct"
            .add_startup_systems(transform_systems())
            .add_systems_to_stage(stage::POST_UPDATE, transform_systems());
    }
}
