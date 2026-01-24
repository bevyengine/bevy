use crate::{
    query::Access,
    world::{
        entity_access::Filtered, unsafe_world_cell::UnsafeEntityCell, All, EntityMut, EntityRef,
    },
};

use thiserror::Error;

/// Provides read-only access to a single [`Entity`] and some of its components,
/// as defined by the contained [`Access`] at runtime. This is an [`EntityRef`]
/// with an [`AsAccess`] of [`Filtered`].
///
/// To define the access when used as a [`QueryData`], use a [`QueryBuilder`] or
/// [`QueryParamBuilder`].
///
/// ```
/// # use bevy_ecs::{prelude::*, world::FilteredEntityRef};
/// #
/// # #[derive(Component)]
/// # struct A;
/// #
/// # let mut world = World::new();
/// # world.spawn(A);
/// #
/// // This gives the `FilteredEntityRef` access to `&A`.
/// let mut query = QueryBuilder::<FilteredEntityRef>::new(&mut world)
///     .data::<&A>()
///     .build();
///
/// let filtered_entity: FilteredEntityRef = query.single(&mut world).unwrap();
/// let component: &A = filtered_entity.get().unwrap();
/// ```
///
/// [`Entity`]: crate::world::Entity
/// [`AsAccess`]: crate::world::AsAccess
/// [`QueryData`]: crate::query::QueryData
/// [`QueryBuilder`]: crate::query::QueryBuilder
/// [`QueryParamBuilder`]: crate::system::QueryParamBuilder
pub type FilteredEntityRef<'w, 's> = EntityRef<'w, Filtered<'s>>;

impl<'w, 's> FilteredEntityRef<'w, 's> {
    /// Consumes `self` and attempts to return an [`EntityRef`] with [`All`]
    /// access.
    ///
    /// # Errors
    ///
    /// Returns [`TryFromFilteredError::MissingReadAllAccess`] if the contained
    /// [`Access`] does not have read access to all components.
    pub fn try_into_all(self) -> Result<EntityRef<'w>, TryFromFilteredError> {
        if !self.access().has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else {
            // SAFETY: `Access::has_read_all` check satisfies the `All` access
            // kind for `EntityRef`.
            Ok(unsafe { EntityRef::new(self.cell, All) })
        }
    }
}

impl<'w> TryFrom<FilteredEntityRef<'w, '_>> for EntityRef<'w> {
    type Error = TryFromFilteredError;

    #[inline]
    fn try_from(entity: FilteredEntityRef<'w, '_>) -> Result<Self, Self::Error> {
        entity.try_into_all()
    }
}

impl<'w> TryFrom<&FilteredEntityRef<'w, '_>> for EntityRef<'w> {
    type Error = TryFromFilteredError;

    #[inline]
    fn try_from(entity: &FilteredEntityRef<'w, '_>) -> Result<Self, Self::Error> {
        entity.try_into_all()
    }
}

/// Variant of [`FilteredEntityMut`] that can be used to create copies of a [`FilteredEntityMut`], as long
/// as the user ensures that these won't cause aliasing violations.
///
/// This can be useful to mutably query multiple components from a single `FilteredEntityMut`.
///
/// ### Example Usage
///
/// ```
/// # use bevy_ecs::{prelude::*, world::{FilteredEntityMut, UnsafeFilteredEntityMut}};
/// #
/// # #[derive(Component)]
/// # struct A;
/// # #[derive(Component)]
/// # struct B;
/// #
/// # let mut world = World::new();
/// # world.spawn((A, B));
/// #
/// // This gives the `FilteredEntityMut` access to `&mut A` and `&mut B`.
/// let mut query = QueryBuilder::<FilteredEntityMut>::new(&mut world)
///     .data::<(&mut A, &mut B)>()
///     .build();
///
/// let mut filtered_entity: FilteredEntityMut = query.single_mut(&mut world).unwrap();
/// let unsafe_filtered_entity = UnsafeFilteredEntityMut::new_readonly(&filtered_entity);
/// // SAFETY: the original FilteredEntityMut accesses `&mut A` and the clone accesses `&mut B`, so no aliasing violations occur.
/// let mut filtered_entity_clone: FilteredEntityMut = unsafe { unsafe_filtered_entity.into_mut() };
/// let a: Mut<A> = filtered_entity.get_mut().unwrap();
/// let b: Mut<B> = filtered_entity_clone.get_mut().unwrap();
/// ```
#[derive(Copy, Clone)]
pub struct UnsafeFilteredEntityMut<'w, 's> {
    entity: UnsafeEntityCell<'w>,
    access: &'s Access,
}

impl<'w, 's> UnsafeFilteredEntityMut<'w, 's> {
    /// Creates a [`UnsafeFilteredEntityMut`] that can be used to have multiple concurrent [`FilteredEntityMut`]s.
    #[inline]
    pub fn new_readonly(filtered_entity_mut: &FilteredEntityMut<'w, 's>) -> Self {
        Self {
            entity: filtered_entity_mut.cell,
            access: filtered_entity_mut.access().0,
        }
    }

    /// Returns a new instance of [`FilteredEntityMut`].
    ///
    /// # Safety
    /// - The user must ensure that no aliasing violations occur when using the returned `FilteredEntityMut`.
    #[inline]
    pub unsafe fn into_mut(self) -> FilteredEntityMut<'w, 's> {
        EntityMut::new(self.entity, Filtered(self.access))
    }
}

/// Provides mutable access to a single [`Entity`] and some of its components,
/// as defined by the contained [`Access`] at runtime. This is an [`EntityMut`]
/// with an [`AsAccess`] of [`Filtered`].
///
/// To define the access when used as a [`QueryData`], use a [`QueryBuilder`] or
/// [`QueryParamBuilder`].
///
/// ```
/// # use bevy_ecs::{prelude::*, world::FilteredEntityMut};
/// #
/// # #[derive(Component)]
/// # struct A;
/// #
/// # let mut world = World::new();
/// # world.spawn(A);
/// #
/// // This gives the `FilteredEntityMut` access to `&mut A`.
/// let mut query = QueryBuilder::<FilteredEntityMut>::new(&mut world)
///     .data::<&mut A>()
///     .build();
///
/// let mut filtered_entity: FilteredEntityMut = query.single_mut(&mut world).unwrap();
/// let component: Mut<A> = filtered_entity.get_mut().unwrap();
/// ```
///
/// Also see [`UnsafeFilteredEntityMut`] for a way to bypass borrow-checker restrictions.
///
/// [`Entity`]: crate::world::Entity
/// [`AsAccess`]: crate::world::AsAccess
/// [`QueryData`]: crate::query::QueryData
/// [`QueryBuilder`]: crate::query::QueryBuilder
/// [`QueryParamBuilder`]: crate::system::QueryParamBuilder
pub type FilteredEntityMut<'w, 's> = EntityMut<'w, Filtered<'s>>;

impl<'w, 's> FilteredEntityMut<'w, 's> {
    /// Consumes `self` and attempts to return an [`EntityMut`] with [`All`] access.
    ///
    /// # Errors
    ///
    /// - Returns [`TryFromFilteredError::MissingReadAllAccess`] if the contained
    ///   [`Access`] does not have read access to all components.
    /// - Returns [`TryFromFilteredError::MissingWriteAllAccess`] if the contained
    ///   [`Access`] does not have write access to all components.
    pub fn try_into_all(self) -> Result<EntityMut<'w>, TryFromFilteredError> {
        if !self.access().has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else if !self.access().has_write_all() {
            Err(TryFromFilteredError::MissingWriteAllAccess)
        } else {
            // SAFETY: `Access::has_read_all` and `Access::has_write_all` checks
            // satisfy the `All` access for `EntityMut`.
            Ok(unsafe { EntityMut::new(self.cell, All) })
        }
    }
}

impl<'w> TryFrom<FilteredEntityMut<'w, '_>> for EntityRef<'w> {
    type Error = TryFromFilteredError;

    #[inline]
    fn try_from(entity: FilteredEntityMut<'w, '_>) -> Result<Self, Self::Error> {
        entity.into_readonly().try_into_all()
    }
}

impl<'w> TryFrom<&'w FilteredEntityMut<'_, '_>> for EntityRef<'w> {
    type Error = TryFromFilteredError;

    #[inline]
    fn try_from(entity: &'w FilteredEntityMut<'_, '_>) -> Result<Self, Self::Error> {
        entity.as_readonly().try_into_all()
    }
}

impl<'w> TryFrom<FilteredEntityMut<'w, '_>> for EntityMut<'w> {
    type Error = TryFromFilteredError;

    #[inline]
    fn try_from(entity: FilteredEntityMut<'w, '_>) -> Result<Self, Self::Error> {
        entity.try_into_all()
    }
}

impl<'w> TryFrom<&'w mut FilteredEntityMut<'_, '_>> for EntityMut<'w> {
    type Error = TryFromFilteredError;

    #[inline]
    fn try_from(entity: &'w mut FilteredEntityMut<'_, '_>) -> Result<Self, Self::Error> {
        entity.reborrow().try_into_all()
    }
}

/// Error type returned by [`TryFrom`] conversions from [`EntityRef`]/[`EntityMut`]
/// entity reference types with [`Filtered`] access to ones with [`All`] access.
#[derive(Error, Debug)]
pub enum TryFromFilteredError {
    /// Error indicating that the filtered entity does not have read access to
    /// all components.
    #[error("Conversion failed, filtered entity ref does not have read access to all components")]
    MissingReadAllAccess,
    /// Error indicating that the filtered entity does not have write access to
    /// all components.
    #[error("Conversion failed, filtered entity ref does not have write access to all components")]
    MissingWriteAllAccess,
}
