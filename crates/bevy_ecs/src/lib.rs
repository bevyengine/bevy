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
        resource::{ChangedRes, FromResources, Local, Res, ResMut, Resource, Resources},
        system::{Commands, IntoSystem, Query, System},
        Added, Bundle, Changed, Component, Entity, In, IntoChainSystem, Mut, Mutated, Or, QuerySet,
        Ref, RefMut, With, Without, World,
    };
}
