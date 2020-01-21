//! Legion aims to be a feature rich high performance ECS library for Rust game projects with minimal boilerplate.
//!
//! # Getting Started
//!
//! ```rust
//! use legion::prelude::*;
//!
//! // Define our entity data types
//! #[derive(Clone, Copy, Debug, PartialEq)]
//! struct Position {
//!     x: f32,
//!     y: f32,
//! }
//!
//! #[derive(Clone, Copy, Debug, PartialEq)]
//! struct Velocity {
//!     dx: f32,
//!     dy: f32,
//! }
//!
//! #[derive(Clone, Copy, Debug, PartialEq)]
//! struct Model(usize);
//!
//! #[derive(Clone, Copy, Debug, PartialEq)]
//! struct Static;
//!
//! // Create a world to store our entities
//! let universe = Universe::new();
//! let mut world = universe.create_world();
//!
//! // Create entities with `Position` and `Velocity` data
//! world.insert(
//!     (),
//!     (0..999).map(|_| (Position { x: 0.0, y: 0.0 }, Velocity { dx: 0.0, dy: 0.0 }))
//! );
//!
//! // Create entities with `Position` data and a tagged with `Model` data and as `Static`
//! // Tags are shared across many entities, and enable further batch processing and filtering use cases
//! world.insert(
//!     (Model(5), Static),
//!     (0..999).map(|_| (Position { x: 0.0, y: 0.0 },))
//! );
//!
//! // Create a query which finds all `Position` and `Velocity` components
//! let mut query = <(Write<Position>, Read<Velocity>)>::query();
//!
//! // Iterate through all entities that match the query in the world
//! for (mut pos, vel) in query.iter_mut(&mut world) {
//!     pos.x += vel.dx;
//!     pos.y += vel.dy;
//! }
//! ```
//!
//! ### Advanced Query Filters
//!
//! The query API can do much more than pull entity data out of the world.
//!
//! Additional data type filters:
//!
//! ```rust
//! # use legion::prelude::*;
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Position {
//! #     x: f32,
//! #     y: f32,
//! # }
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Velocity {
//! #     dx: f32,
//! #     dy: f32,
//! # }
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Model(usize);
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Static;
//! # let universe = Universe::new();
//! # let mut world = universe.create_world();
//! // It is possible to specify that entities must contain data beyond that being fetched
//! let mut query = Read::<Position>::query()
//!     .filter(component::<Velocity>());
//! for position in query.iter(&mut world) {
//!     // these entities also have `Velocity`
//! }
//! ```
//!
//! Filter boolean operations:
//!
//! ```rust
//! # use legion::prelude::*;
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Position {
//! #     x: f32,
//! #     y: f32,
//! # }
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Velocity {
//! #     dx: f32,
//! #     dy: f32,
//! # }
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Model(usize);
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Static;
//! # let universe = Universe::new();
//! # let mut world = universe.create_world();
//! // Filters can be combined with boolean operators
//! let mut query = Read::<Position>::query()
//!     .filter(tag::<Static>() | !component::<Velocity>());
//! for position in query.iter(&mut world) {
//!     // these entities are also either marked as `Static`, or do *not* have a `Velocity`
//! }
//! ```
//!
//! Filter by tag data value:
//!
//! ```rust
//! # use legion::prelude::*;
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Position {
//! #     x: f32,
//! #     y: f32,
//! # }
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Velocity {
//! #     dx: f32,
//! #     dy: f32,
//! # }
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Model(usize);
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Static;
//! # let universe = Universe::new();
//! # let mut world = universe.create_world();
//! // Filters can filter by specific tag values
//! let mut query = Read::<Position>::query()
//!     .filter(tag_value(&Model(3)));
//! for position in query.iter(&mut world) {
//!     // these entities all have tag value `Model(3)`
//! }
//! ```
//!
//! Change detection:
//!
//! ```rust
//! # use legion::prelude::*;
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Position {
//! #     x: f32,
//! #     y: f32,
//! # }
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Velocity {
//! #     dx: f32,
//! #     dy: f32,
//! # }
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Model(usize);
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Static;
//! # let universe = Universe::new();
//! # let mut world = universe.create_world();
//! // Queries can perform coarse-grained change detection, rejecting entities who's data
//! // has not changed since the last time the query was iterated.
//! let mut query = <(Read<Position>, Tagged<Model>)>::query()
//!     .filter(changed::<Position>());
//! for (pos, model) in query.iter(&mut world) {
//!     // entities who have changed position
//! }
//! ```
//!
//! ### Content Streaming
//!
//! Entities can be loaded and initialized in a background `World` on separate threads and then
//! when ready, merged into the main `World` near instantaneously.
//!
//! ```rust
//! # use legion::prelude::*;
//! let universe = Universe::new();
//! let mut world_a = universe.create_world();
//! let mut world_b = universe.create_world();
//!
//! // Merge all entities from `world_b` into `world_a`
//! // Entity IDs are guarenteed to be unique across worlds and will
//! // remain unchanged across the merge.
//! world_a.merge(world_b);
//! ```
//!
//! ### Chunk Iteration
//!
//! Entity data is allocated in blocks called "chunks", each approximately containing 64KiB of data.
//! The query API exposes each chunk via 'iter_chunk'. As all entities in a chunk are guarenteed to contain the same set of entity
//! data and shared data values, it is possible to do batch processing via the chunk API.
//!
//! ```rust
//! # use legion::prelude::*;
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Transform;
//! # #[derive(Clone, Copy, Debug, PartialEq)]
//! # struct Model(usize);
//! # let universe = Universe::new();
//! # let mut world = universe.create_world();
//! fn render_instanced(model: &Model, transforms: &[Transform]) {
//!     // pass `transforms` pointer to graphics API to load into constant buffer
//!     // issue instanced draw call with model data and transforms
//! }
//!
//! let mut query = Read::<Transform>::query()
//!     .filter(tag::<Model>());
//!
//! for chunk in query.iter_chunks_mut(&mut world) {
//!     // get the chunk's model
//!     let model: &Model = chunk.tag().unwrap();
//!
//!     // get a (runtime borrow checked) slice of transforms
//!     let transforms = chunk.components::<Transform>().unwrap();
//!
//!     // give the model and transform slice to our renderer
//!     render_instanced(model, &transforms);
//! }
//! ```
//!
//! # Feature Flags
//!
//!  * `par-iter`: Enables parallel APIs on queries (enabled by default).
//!  * `par-schedule`: Configures system schedulers to try and run systems in parallel where possible (enabled by default).
//!  * `log`: Configures `tracing` to redirect events to the `log` crate. This is a convenience feature for applications
//!  that use `log` and do not wish to interact with `tracing`.
//!  * `events`: Enables eventing APIs on worlds (enabled by default).
#![allow(dead_code)]

pub mod borrow;
pub mod command;
#[cfg(feature = "serde-1")]
pub mod de;
pub mod entity;
pub mod event;
pub mod filter;
pub mod iterator;
pub mod query;
pub mod resource;
pub mod schedule;
#[cfg(feature = "serde-1")]
pub mod ser;
pub mod storage;
pub mod system;
pub mod world;

mod cons;
mod tuple;
mod zip;

pub use bit_set;

pub mod prelude {
    pub use crate::command::CommandBuffer;
    pub use crate::entity::Entity;
    pub use crate::event::Event;
    pub use crate::filter::filter_fns::*;
    pub use crate::query::{IntoQuery, Query, Read, Tagged, TryRead, TryWrite, Write};
    pub use crate::resource::{ResourceSet, Resources};
    pub use crate::schedule::{Executor, Runnable, Schedulable, Schedule};
    pub use crate::system::{System, SystemBuilder};
    pub use crate::world::{Universe, World};
    pub use bit_set::BitSet;
}
