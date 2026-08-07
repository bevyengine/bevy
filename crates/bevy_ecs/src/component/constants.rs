//! Constant components included in every world.

/// `usize` for the [`Add`](crate::lifecycle::Add) component used in lifecycle observers.
pub const ADD: usize = 0;
/// `usize` for the [`Insert`](crate::lifecycle::Insert) component used in lifecycle observers.
pub const INSERT: usize = 1;
/// `usize` for the [`Mutate`](crate::lifecycle::Mutate) component used in lifecycle observers.
pub const MUTATE: usize = 2;
/// `usize` for the [`Discard`](crate::lifecycle::Discard) component used in lifecycle observers.
pub const DISCARD: usize = 3;
/// `usize` for the [`Remove`](crate::lifecycle::Remove) component used in lifecycle observers.
pub const REMOVE: usize = 4;
/// `usize` for [`Despawn`](crate::lifecycle::Despawn) component used in lifecycle observers.
pub const DESPAWN: usize = 5;
/// `usize` of the [`IsResource`](crate::resource::IsResource) component used to mark entities with resources.
pub const IS_RESOURCE: usize = 6;
/// `usize` for [`ArchetypeCreated`](crate::archetype::ArchetypeCreated) component used as an observer event key.
pub(crate) const ARCHETYPE_CREATED: usize = 7;
