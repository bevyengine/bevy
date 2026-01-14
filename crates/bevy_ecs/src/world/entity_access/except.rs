use crate::{
    bundle::Bundle,
    world::{EntityMut, EntityRef, Except, Filtered, FilteredEntityMut, FilteredEntityRef},
};

/// Provides read-only access to a single [`Entity`] and all its components,
/// except those mentioned in the [`Bundle`] `B` at compile time. This is an
/// [`EntityRef`] with an [`AccessScope`] of [`Except`].
///
/// [`Entity`]: crate::world::Entity
/// [`AccessScope`]: crate::world::AccessScope
pub type EntityRefExcept<'w, 's, B> = EntityRef<'w, Except<'s, B>>;

impl<'w, 's, B: Bundle> EntityRefExcept<'w, 's, B> {
    /// Consumes `self` and returns a [`FilteredEntityRef`] with the same access
    /// permissions.
    pub fn into_filtered(self) -> FilteredEntityRef<'w, 's> {
        // SAFETY:
        // - Read permissions of the `Except` scope are preserved in the `Filtered` scope.
        unsafe { EntityRef::new(self.cell, Filtered(self.scope().0)) }
    }
}

impl<'w, 's, B: Bundle> From<EntityRefExcept<'w, 's, B>> for FilteredEntityRef<'w, 's> {
    #[inline]
    fn from(entity: EntityRefExcept<'w, 's, B>) -> Self {
        entity.into_filtered()
    }
}

impl<'w, 's, B: Bundle> From<&EntityRefExcept<'w, 's, B>> for FilteredEntityRef<'w, 's> {
    #[inline]
    fn from(entity: &EntityRefExcept<'w, 's, B>) -> Self {
        entity.into_filtered()
    }
}

/// Provides mutable access to a single [`Entity`] and all its components,
/// except those mentioned in the [`Bundle`] `B` at compile time. This is an
/// [`EntityMut`] with an [`AccessScope`] of [`Except`].
///
/// This is a rather niche type that should only be used if you need access to
/// *all* components of an entity, while still allowing you to consult other
/// queries that might match entities that this query also matches. If you don't
/// need access to all components, prefer a standard query with a
/// [`Without`](`crate::query::Without`) filter.
///
/// [`Entity`]: crate::world::Entity
/// [`AccessScope`]: crate::world::AccessScope
pub type EntityMutExcept<'w, 's, B> = EntityMut<'w, Except<'s, B>>;

impl<'w, 's, B: Bundle> EntityMutExcept<'w, 's, B> {
    /// Consumes `self` and returns a [`FilteredEntityMut`] with the same access
    /// permissions.
    #[inline]
    pub fn into_filtered(self) -> FilteredEntityMut<'w, 's> {
        // SAFETY:
        // - Read and write permissions of the `Except` scope are preserved in
        //   the `Filtered` scope.
        // - Consuming `self` ensures there are no other accesses.
        unsafe { EntityMut::new(self.cell, Filtered(self.scope().0)) }
    }
}

impl<'w, 's, B: Bundle> From<EntityMutExcept<'w, 's, B>> for FilteredEntityRef<'w, 's> {
    #[inline]
    fn from(entity: EntityMutExcept<'w, 's, B>) -> Self {
        entity.into_readonly().into_filtered()
    }
}

impl<'w, 's, B: Bundle> From<&'w EntityMutExcept<'_, 's, B>> for FilteredEntityRef<'w, 's> {
    #[inline]
    fn from(entity: &'w EntityMutExcept<'_, 's, B>) -> Self {
        entity.as_readonly().into_filtered()
    }
}

impl<'w, 's, B: Bundle> From<EntityMutExcept<'w, 's, B>> for FilteredEntityMut<'w, 's> {
    #[inline]
    fn from(entity: EntityMutExcept<'w, 's, B>) -> Self {
        entity.into_filtered()
    }
}

impl<'w, 's, B: Bundle> From<&'w mut EntityMutExcept<'_, 's, B>> for FilteredEntityMut<'w, 's> {
    #[inline]
    fn from(entity: &'w mut EntityMutExcept<'_, 's, B>) -> Self {
        entity.reborrow().into_filtered()
    }
}
