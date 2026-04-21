//! Constant components included in every world.

/// `usize` for the [`Add`](crate::lifecycle::Add) component used in lifecycle observers.
pub const ADD: u32 = 0;
/// `usize` for the [`Insert`](crate::lifecycle::Insert) component used in lifecycle observers.
pub const INSERT: u32 = 1;
/// `usize` for the [`Discard`](crate::lifecycle::Discard) component used in lifecycle observers.
pub const DISCARD: u32 = 2;
/// `usize` for the [`Remove`](crate::lifecycle::Remove) component used in lifecycle observers.
pub const REMOVE: u32 = 3;
/// `usize` for [`Despawn`](crate::lifecycle::Despawn) component used in lifecycle observers.
pub const DESPAWN: u32 = 4;
/// `usize` of the [`IsResource`](crate::resource::IsResource) component used to mark entities with resources.
pub const IS_RESOURCE: u32 = 5;
