//! A source inspecting a separate running app over the Bevy Remote Protocol.
//!
//! The remote entities and their components are mirrored into a separate [`World`], the
//! [`RemoteWorld`], at their remote ids. The entity tree and the details panel read that world
//! instead of the local one, so no local hook, observer or system ever sees remote data. See
//! [`world`] for how values are inserted.
//!
//! The entity tree is polled with `world.query` for the components it needs only: [`Name`],
//! [`ChildOf`] and the label-defining components. A full poll reading every component runs when
//! entities the inspector has not seen appear, and at least every 10 seconds. It tells which
//! entities hold a reflected component, since the others, such as observers and systems, are not
//! mirrored. The components of the selected entity are fetched separately, see [`details`].
//! Answers are deserialized off the main thread, and only values whose JSON changed are written.
//!
//! Remote entities with the [`Disabled`] component are not shown, since `world.query` skips them.
//!
//! [`Name`]: bevy_ecs::name::Name
//! [`ChildOf`]: bevy_ecs::hierarchy::ChildOf
//! [`Disabled`]: bevy_ecs::entity_disabling::Disabled
//! [`World`]: bevy_ecs::world::World

pub mod details;
mod source;
pub mod world;

pub(crate) use source::is_remote;
#[cfg(test)]
pub(crate) use source::tests;
pub use source::{
    poll_remote_connection, sync_remote_source, sync_remote_world, RemoteConnection,
    RemoteConnectionState, RemoteSnapshot, RemoteSource,
};
pub use world::{RemoteComponents, RemoteWorld};
