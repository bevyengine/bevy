pub use glam as math;
pub use legion as ecs;

pub mod components;
pub mod hierarchy_maintenance_system;
pub mod local_to_parent_system;
pub mod local_to_world_propagate_system;
pub mod local_to_world_system;
pub mod transform_system_bundle;

pub mod prelude {
    pub use crate::{
        components::*, hierarchy_maintenance_system, local_to_parent_system,
        local_to_world_propagate_system, local_to_world_system, transform_system_bundle,
    };
}
