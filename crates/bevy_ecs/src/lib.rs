pub use hecs::{*, Query as HecsQuery};

mod into_system;
mod resource_query;
mod resources;
mod commands;
mod schedule;
mod system;
mod world_builder;
#[cfg(feature = "profiler")]
pub mod profiler;

pub use into_system::{IntoForEachSystem, IntoQuerySystem, Query, ThreadLocalSystem};
pub use resource_query::{Res, ResMut, ResourceQuery, Local, FetchResource};
pub use resources::{Resources, FromResources, Resource};
pub use commands::{Commands, CommandsInternal};
pub use schedule::Schedule;
pub use system::{System, SystemId};
pub use world_builder::{WorldBuilder, WorldBuilderSource};