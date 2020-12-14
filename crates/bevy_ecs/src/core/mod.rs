/// Imagine macro parameters, but more like those Russian dolls.
///
/// Calls m!(A, B, C), m!(A, B), m!(B), and m!() for i.e. (m, A, B, C)
/// where m is any macro, for any number of parameters.
#[macro_export]
macro_rules! smaller_tuples_too {
    ($m: ident, $ty: ident) => {
        $m!{$ty}
        $m!{}
    };
    ($m: ident, $ty: ident, $($tt: ident),*) => {
        $m!{$ty, $($tt),*}
        smaller_tuples_too!{$m, $($tt),*}
    };
}

mod access;
mod archetype;
mod borrow;
mod bundle;
mod entities;
mod entity_builder;
mod entity_map;
mod filter;
mod query;
mod serde;
mod world;
mod world_builder;

pub use access::{ArchetypeComponent, QueryAccess, TypeAccess};
pub use archetype::{Archetype, ComponentFlags, TypeState};
pub use borrow::{AtomicBorrow, Ref, RefMut};
pub use bundle::{Bundle, DynamicBundle, MissingComponent};
pub use entities::{Entity, EntityReserver, Location, NoSuchEntity};
pub use entity_builder::{BuiltEntity, EntityBuilder};
pub use entity_map::*;
pub use filter::{Added, Changed, EntityFilter, Mutated, Or, QueryFilter, With, Without};
pub use query::{Batch, BatchedIter, Mut, QueryIter, ReadOnlyFetch, WorldQuery};
pub use world::{ArchetypesGeneration, Component, ComponentError, SpawnBatchIter, World};
pub use world_builder::*;

// Unstable implementation details needed by the macros
#[doc(hidden)]
pub use archetype::TypeInfo;
#[doc(hidden)]
pub use bevy_utils;
#[doc(hidden)]
pub use query::Fetch;
