// Copyright 2019 Google LLC
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     https://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

// modified by Bevy contributors

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
pub use query::{Batch, BatchedIter, Flags, Mut, QueryIter, ReadOnlyFetch, WorldQuery};
pub use world::{ArchetypesGeneration, Component, ComponentError, SpawnBatchIter, World};
pub use world_builder::*;

// Unstable implementation details needed by the macros
#[doc(hidden)]
pub use archetype::TypeInfo;
#[doc(hidden)]
pub use bevy_utils;
#[doc(hidden)]
pub use query::Fetch;
