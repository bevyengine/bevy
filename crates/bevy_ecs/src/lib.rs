mod core;
mod resource;
mod schedule;
mod system;

pub use crate::core::*;
pub use bevy_ecs_macros::*;
pub use lazy_static;
pub use resource::*;
pub use schedule::*;
pub use system::{Query, *};

pub mod prelude {
    pub use crate::{
        core::WorldBuilderSource,
        resource::{ChangedRes, FromResources, Res, ResMut, Resource, Resources},
        schedule::{Schedule, State, StateStage, SystemStage},
        system::{AsSystem, Commands, IntoSystem, Local, Query, System},
        Added, Bundle, Changed, Component, Entity, Flags, In, IntoChainSystem, Mutated, Or,
        QuerySet, With, Without, World,
    };
}
