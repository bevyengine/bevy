//! Constant components included in every world.

/// `u32` for the [`Add`](crate::lifecycle::Add) component used in lifecycle observers.
pub const ADD: u32 = 0;
/// `u32` for the [`Insert`](crate::lifecycle::Insert) component used in lifecycle observers.
pub const INSERT: u32 = 1;
/// `u32` for the [`Discard`](crate::lifecycle::Discard) component used in lifecycle observers.
pub const DISCARD: u32 = 2;
/// `u32` for the [`Remove`](crate::lifecycle::Remove) component used in lifecycle observers.
pub const REMOVE: u32 = 3;
/// `u32` for [`Despawn`](crate::lifecycle::Despawn) component used in lifecycle observers.
pub const DESPAWN: u32 = 4;
/// `u32` of the [`IsResource`](crate::resource::IsResource) component used to mark entities with resources.
pub const IS_RESOURCE: u32 = 5;
