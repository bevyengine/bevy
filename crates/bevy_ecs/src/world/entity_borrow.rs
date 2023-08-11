use crate::world::unsafe_world_cell::UnsafeEntityCell;

/// Provides read-only access to a single entity and all of its components.
///
/// Contrast with [`EntityRef`], which provides access to the entire world
/// in addition to an entity.
pub struct EntityBorrow<'w>(UnsafeEntityCell<'w>);

impl<'w> EntityBorrow<'w> {
    /// # Safety
    /// - `cell` must have permission to read every component of the entity.
    /// - No mutable accesses to any of the entity's components may exist
    ///   at the same time as the returned [`EntityBorrow`].
    pub(crate) unsafe fn new(cell: UnsafeEntityCell<'w>) -> Self {
        Self(cell)
    }
}

/// Provides mutable access to a single entity and all of its components.
///
/// Unlike [`EntityMut`], this type allows disjoint accesses to multiple entities at once.
///
/// [`EntityMut`]: super::EntityMut
pub struct EntityBorrowMut<'w>(UnsafeEntityCell<'w>);

impl<'w> EntityBorrowMut<'w> {
    /// # Safety
    /// - `cell` must have permission to mutate every component of the entity.
    /// - No accesses to any of the entity's components may exist
    ///   at the same time as the returned [`EntityBorrowMut`].
    pub(crate) unsafe fn new(cell: UnsafeEntityCell<'w>) -> Self {
        Self(cell)
    }
}
