pub use hecs::{Query as HecsQuery, *};
mod resource;
mod schedule;
mod system;
mod world;

pub use resource::*;
pub use schedule::*;
pub use system::{*, Query};
pub use world::*;

pub mod prelude {
    pub use crate::{
        resource::{FromResources, Local, Res, ResMut, Resource, Resources},
        system::{
            Commands, IntoForEachSystem, IntoQuerySystem, IntoThreadLocalSystem, Query, System,
        },
        world::{WorldBuilderSource, Track},
        Bundle, Component, Entity, Ref, RefMut, With, Without, World,
    };
}
