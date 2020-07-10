pub use glam as math;

pub mod child_builder;
pub mod components;
pub mod hierarchy_maintenance_system;
pub mod local_transform_systems;
pub mod transform_propagate_system;
pub mod transform_systems;
pub mod world_child_builder;

pub mod prelude {
    pub use crate::{build_systems, child_builder::*, components::*, world_child_builder::*};
}

use bevy_ecs::{IntoQuerySystem, System};

// TODO: make this a plugin
pub fn build_systems() -> Vec<Box<dyn System>> {
    let mut all_systems = Vec::with_capacity(5);

    all_systems.append(&mut hierarchy_maintenance_system::hierarchy_maintenance_systems());
    all_systems.append(&mut local_transform_systems::local_transform_systems());
    all_systems.append(&mut transform_systems::transform_systems());
    all_systems.push(transform_propagate_system::transform_propagate_system.system());

    all_systems
}
