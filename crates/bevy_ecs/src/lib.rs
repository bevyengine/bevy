pub use bevy_hecs::{Query as HecsQuery, *};
mod resource;
mod schedule;
mod system;
mod world;

pub use resource::*;
pub use schedule::*;
pub use system::{Query, *};
pub use world::*;

pub mod prelude {
    pub use crate::{
        resource::{ChangedRes, FromResources, Local, Res, ResMut, Resource, Resources},
        system::{Commands, IntoSystem, IntoThreadLocalSystem, Query, System},
        world::WorldBuilderSource,
        Added, Bundle, Changed, Component, Entity, Mut, Mutated, Or, QuerySet, Ref, RefMut, With,
        Without, World,
    };
}
