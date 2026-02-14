//! Constant components included in every world.

/// `usize` for the [`Add`](crate::lifecycle::Add) component used in lifecycle observers.
pub const ADD: usize = 0;
/// `usize` for the [`Insert`](crate::lifecycle::Insert) component used in lifecycle observers.
pub const INSERT: usize = 1;
/// `usize` for the [`Discard`](crate::lifecycle::Discard) component used in lifecycle observers.
pub const DISCARD: usize = 2;
/// `usize` for the [`Remove`](crate::lifecycle::Remove) component used in lifecycle observers.
pub const REMOVE: usize = 3;
/// `usize` for [`Despawn`](crate::lifecycle::Despawn) component used in lifecycle observers.
pub const DESPAWN: usize = 4;
/// `usize` for the [`BeforeAdd`](crate::lifecycle::BeforeAdd) component used in lifecycle observers.
pub const BEFORE_ADD: usize = 5;
/// `usize` for the [`AfterRemove`](crate::lifecycle::AfterRemove) component used in lifecycle observers.
pub const AFTER_REMOVE: usize = 6;
/// `usize` of the [`IsResource`](crate::resource::IsResource) component used to mark entities with resources.
pub const IS_RESOURCE: usize = 7;
