pub use legion as ecs;
pub use nalgebra as math;

pub mod components;
pub mod hierarchy_maintenance_system;
pub mod local_to_parent_system;
pub mod local_to_world_propagate_system;
pub mod local_to_world_system;
pub mod transform_system_bundle;

pub mod prelude {
    pub use crate::components::*;
    pub use crate::hierarchy_maintenance_system;
    pub use crate::local_to_parent_system;
    pub use crate::local_to_world_propagate_system;
    pub use crate::local_to_world_system;
    pub use crate::transform_system_bundle;
}
