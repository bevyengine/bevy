use crate::{
    archetype::Archetype,
    change_detection::{ComponentTicks, MaybeLocation, MutUntyped, Tick},
    component::{Component, ComponentId, Mutable},
    entity::{ContainsEntity, Entity, EntityEquivalent, EntityLocation},
    query::Access,
    world::{unsafe_world_cell::UnsafeEntityCell, EntityMut, EntityRef, Mut, Ref},
};

use bevy_ptr::Ptr;
use core::{
    any::TypeId,
    cmp::Ordering,
    hash::{Hash, Hasher},
};
use thiserror::Error;

/// Provides read-only access to a single entity and some of its components defined by the contained [`Access`].
///
/// To define the access when used as a [`QueryData`](crate::query::QueryData),
/// use a [`QueryBuilder`](crate::query::QueryBuilder) or [`QueryParamBuilder`](crate::system::QueryParamBuilder).
/// The [`FilteredEntityRef`] must be the entire [`QueryData`](crate::query::QueryData), and not nested inside a tuple with other data.
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
#[derive(Clone, Copy)]
pub struct FilteredEntityRef<'w, 's> {
    entity: UnsafeEntityCell<'w>,
    access: &'s Access,
}

impl<'w, 's> FilteredEntityRef<'w, 's> {
    /// # Safety
    /// - No `&mut World` can exist from the underlying `UnsafeWorldCell`
    /// - If `access` takes read access to a component no mutable reference to that
    ///   component can exist at the same time as the returned [`FilteredEntityMut`]
    /// - If `access` takes any access for a component `entity` must have that component.
    #[inline]
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>, access: &'s Access) -> Self {
        Self { entity, access }
    }

    /// Consumes self and returns read-only access to the entity and *all* of
    /// its components, with the world `'w` lifetime. Returns an error if the
    /// access does not include read access to all components.
    ///
    /// # Errors
    ///
    /// - [`TryFromFilteredError::MissingReadAllAccess`] - if the access does not include read access to all components.
    #[inline]
    pub fn try_into_all(self) -> Result<EntityRef<'w>, TryFromFilteredError> {
        if !self.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else {
            // SAFETY: check above guarantees read-only access to all components of the entity.
            Ok(unsafe { EntityRef::new(self.entity) })
        }
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity.id()
    }

    /// Gets metadata indicating the location where the current entity is stored.
    #[inline]
    pub fn location(&self) -> EntityLocation {
        self.entity.location()
    }

    /// Returns the archetype that the current entity belongs to.
    #[inline]
    pub fn archetype(&self) -> &Archetype {
        self.entity.archetype()
    }

    /// Returns a reference to the underlying [`Access`].
    #[inline]
    pub fn access(&self) -> &Access {
        self.access
    }

    /// Returns `true` if the current entity has a component of type `T`.
    /// Otherwise, this returns `false`.
    ///
    /// ## Notes
    ///
    /// If you do not know the concrete type of a component, consider using
    /// [`Self::contains_id`] or [`Self::contains_type_id`].
    #[inline]
    pub fn contains<T: Component>(&self) -> bool {
        self.contains_type_id(TypeId::of::<T>())
    }

    /// Returns `true` if the current entity has a component identified by `component_id`.
    /// Otherwise, this returns false.
    ///
    /// ## Notes
    ///
    /// - If you know the concrete type of the component, you should prefer [`Self::contains`].
    /// - If you know the component's [`TypeId`] but not its [`ComponentId`], consider using
    ///   [`Self::contains_type_id`].
    #[inline]
    pub fn contains_id(&self, component_id: ComponentId) -> bool {
        self.entity.contains_id(component_id)
    }

    /// Returns `true` if the current entity has a component with the type identified by `type_id`.
    /// Otherwise, this returns false.
    ///
    /// ## Notes
    ///
    /// - If you know the concrete type of the component, you should prefer [`Self::contains`].
    /// - If you have a [`ComponentId`] instead of a [`TypeId`], consider using [`Self::contains_id`].
    #[inline]
    pub fn contains_type_id(&self, type_id: TypeId) -> bool {
        self.entity.contains_type_id(type_id)
    }

    /// Gets access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'w T> {
        let id = self
            .entity
            .world()
            .components()
            .get_valid_id(TypeId::of::<T>())?;
        self.access
            .has_component_read(id)
            // SAFETY: We have read access
            .then(|| unsafe { self.entity.get() })
            .flatten()
    }

    /// Gets access to the component of type `T` for the current entity,
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_ref<T: Component>(&self) -> Option<Ref<'w, T>> {
        let id = self
            .entity
            .world()
            .components()
            .get_valid_id(TypeId::of::<T>())?;
        self.access
            .has_component_read(id)
            // SAFETY: We have read access
            .then(|| unsafe { self.entity.get_ref() })
            .flatten()
    }

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    #[inline]
    pub fn get_change_ticks<T: Component>(&self) -> Option<ComponentTicks> {
        let id = self
            .entity
            .world()
            .components()
            .get_valid_id(TypeId::of::<T>())?;
        self.access
            .has_component_read(id)
            // SAFETY: We have read access
            .then(|| unsafe { self.entity.get_change_ticks::<T>() })
            .flatten()
    }

    /// Retrieves the change ticks for the given [`ComponentId`]. This can be useful for implementing change
    /// detection in custom runtimes.
    ///
    /// **You should prefer to use the typed API [`Self::get_change_ticks`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    #[inline]
    pub fn get_change_ticks_by_id(&self, component_id: ComponentId) -> Option<ComponentTicks> {
        self.access
            .has_component_read(component_id)
            // SAFETY: We have read access
            .then(|| unsafe { self.entity.get_change_ticks_by_id(component_id) })
            .flatten()
    }

    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`Self::get`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`FilteredEntityRef::get`], this returns a raw pointer to the component,
    /// which is only valid while the [`FilteredEntityRef`] is alive.
    #[inline]
    pub fn get_by_id(&self, component_id: ComponentId) -> Option<Ptr<'w>> {
        self.access
            .has_component_read(component_id)
            // SAFETY: We have read access
            .then(|| unsafe { self.entity.get_by_id(component_id) })
            .flatten()
    }

    /// Returns the source code location from which this entity has been spawned.
    pub fn spawned_by(&self) -> MaybeLocation {
        self.entity.spawned_by()
    }

    /// Returns the [`Tick`] at which this entity has been spawned.
    pub fn spawn_tick(&self) -> Tick {
        self.entity.spawn_tick()
    }
}

impl<'a> TryFrom<FilteredEntityRef<'a, '_>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    fn try_from(entity: FilteredEntityRef<'a, '_>) -> Result<Self, Self::Error> {
        entity.try_into_all()
    }
}

impl<'a> TryFrom<&FilteredEntityRef<'a, '_>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    fn try_from(entity: &FilteredEntityRef<'a, '_>) -> Result<Self, Self::Error> {
        entity.try_into_all()
    }
}

impl PartialEq for FilteredEntityRef<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.entity() == other.entity()
    }
}

impl Eq for FilteredEntityRef<'_, '_> {}

impl PartialOrd for FilteredEntityRef<'_, '_> {
    /// [`FilteredEntityRef`]'s comparison trait implementations match the underlying [`Entity`],
    /// and cannot discern between different worlds.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FilteredEntityRef<'_, '_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.entity().cmp(&other.entity())
    }
}

impl Hash for FilteredEntityRef<'_, '_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity().hash(state);
    }
}

impl ContainsEntity for FilteredEntityRef<'_, '_> {
    fn entity(&self) -> Entity {
        self.id()
    }
}

// SAFETY: This type represents one Entity. We implement the comparison traits based on that Entity.
unsafe impl EntityEquivalent for FilteredEntityRef<'_, '_> {}

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
            entity: filtered_entity_mut.entity,
            access: filtered_entity_mut.access,
        }
    }

    /// Returns a new instance of [`FilteredEntityMut`].
    ///
    /// # Safety
    /// - The user must ensure that no aliasing violations occur when using the returned `FilteredEntityMut`.
    #[inline]
    pub unsafe fn into_mut(self) -> FilteredEntityMut<'w, 's> {
        // SAFETY: Upheld by caller.
        unsafe { FilteredEntityMut::new(self.entity, self.access) }
    }
}

/// Provides mutable access to a single entity and some of its components defined by the contained [`Access`].
///
/// To define the access when used as a [`QueryData`](crate::query::QueryData),
/// use a [`QueryBuilder`](crate::query::QueryBuilder) or [`QueryParamBuilder`](crate::system::QueryParamBuilder).
/// The `FilteredEntityMut` must be the entire `QueryData`, and not nested inside a tuple with other data.
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
pub struct FilteredEntityMut<'w, 's> {
    entity: UnsafeEntityCell<'w>,
    access: &'s Access,
}

impl<'w, 's> FilteredEntityMut<'w, 's> {
    /// # Safety
    /// - No `&mut World` can exist from the underlying `UnsafeWorldCell`
    /// - If `access` takes read access to a component no mutable reference to that
    ///   component can exist at the same time as the returned [`FilteredEntityMut`]
    /// - If `access` takes write access to a component, no reference to that component
    ///   may exist at the same time as the returned [`FilteredEntityMut`]
    /// - If `access` takes any access for a component `entity` must have that component.
    #[inline]
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>, access: &'s Access) -> Self {
        Self { entity, access }
    }

    /// Returns a new instance with a shorter lifetime.
    /// This is useful if you have `&mut FilteredEntityMut`, but you need `FilteredEntityMut`.
    #[inline]
    pub fn reborrow(&mut self) -> FilteredEntityMut<'_, 's> {
        // SAFETY:
        // - We have exclusive access to the entire entity and the applicable components.
        // - `&mut self` ensures there are no other accesses to the applicable components.
        unsafe { Self::new(self.entity, self.access) }
    }

    /// Consumes `self` and returns read-only access to all of the entity's
    /// components, with the world `'w` lifetime.
    #[inline]
    pub fn into_readonly(self) -> FilteredEntityRef<'w, 's> {
        // SAFETY:
        // - We have exclusive access to the entire entity and the applicable components.
        // - Consuming `self` ensures there are no other accesses to the applicable components.
        unsafe { FilteredEntityRef::new(self.entity, self.access) }
    }

    /// Gets read-only access to all of the entity's components.
    #[inline]
    pub fn as_readonly(&self) -> FilteredEntityRef<'_, 's> {
        // SAFETY:
        // - We have exclusive access to the entire entity and the applicable components.
        // - `&self` ensures there are no mutable accesses to the applicable components.
        unsafe { FilteredEntityRef::new(self.entity, self.access) }
    }

    /// Consumes self and returns mutable access to the entity and *all* of
    /// its components, with the world `'w` lifetime. Returns an error if the
    /// access does not include read and write access to all components.
    ///
    /// # Errors
    ///
    /// - [`TryFromFilteredError::MissingReadAllAccess`] - if the access does not include read access to all components.
    /// - [`TryFromFilteredError::MissingWriteAllAccess`] - if the access does not include write access to all components.
    #[inline]
    pub fn try_into_all(self) -> Result<EntityMut<'w>, TryFromFilteredError> {
        if !self.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else if !self.access.has_write_all() {
            Err(TryFromFilteredError::MissingWriteAllAccess)
        } else {
            // SAFETY: check above guarantees exclusive access to all components of the entity.
            Ok(unsafe { EntityMut::new(self.entity) })
        }
    }

    /// Get access to the underlying [`UnsafeEntityCell`].
    #[inline]
    pub fn as_unsafe_entity_cell(&mut self) -> UnsafeEntityCell<'_> {
        self.entity
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity.id()
    }

    /// Gets metadata indicating the location where the current entity is stored.
    #[inline]
    pub fn location(&self) -> EntityLocation {
        self.entity.location()
    }

    /// Returns the archetype that the current entity belongs to.
    #[inline]
    pub fn archetype(&self) -> &Archetype {
        self.entity.archetype()
    }

    /// Returns a reference to the underlying [`Access`].
    #[inline]
    pub fn access(&self) -> &Access {
        self.access
    }

    /// Returns `true` if the current entity has a component of type `T`.
    /// Otherwise, this returns `false`.
    ///
    /// ## Notes
    ///
    /// If you do not know the concrete type of a component, consider using
    /// [`Self::contains_id`] or [`Self::contains_type_id`].
    #[inline]
    pub fn contains<T: Component>(&self) -> bool {
        self.contains_type_id(TypeId::of::<T>())
    }

    /// Returns `true` if the current entity has a component identified by `component_id`.
    /// Otherwise, this returns false.
    ///
    /// ## Notes
    ///
    /// - If you know the concrete type of the component, you should prefer [`Self::contains`].
    /// - If you know the component's [`TypeId`] but not its [`ComponentId`], consider using
    ///   [`Self::contains_type_id`].
    #[inline]
    pub fn contains_id(&self, component_id: ComponentId) -> bool {
        self.entity.contains_id(component_id)
    }

    /// Returns `true` if the current entity has a component with the type identified by `type_id`.
    /// Otherwise, this returns false.
    ///
    /// ## Notes
    ///
    /// - If you know the concrete type of the component, you should prefer [`Self::contains`].
    /// - If you have a [`ComponentId`] instead of a [`TypeId`], consider using [`Self::contains_id`].
    #[inline]
    pub fn contains_type_id(&self, type_id: TypeId) -> bool {
        self.entity.contains_type_id(type_id)
    }

    /// Gets access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'_ T> {
        self.as_readonly().get()
    }

    /// Gets access to the component of type `T` for the current entity,
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_ref<T: Component>(&self) -> Option<Ref<'_, T>> {
        self.as_readonly().get_ref()
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T` or if
    /// the access does not include write access to `T`.
    #[inline]
    pub fn get_mut<T: Component<Mutability = Mutable>>(&mut self) -> Option<Mut<'_, T>> {
        // SAFETY: we use a mutable reference to self, so we cannot use the `FilteredEntityMut` to access
        // another component
        unsafe { self.get_mut_unchecked() }
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T` or if
    /// the access does not include write access to `T`.
    ///
    /// This only requires `&self`, and so may be used to get mutable access to multiple components.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::FilteredEntityMut};
    /// #
    /// #[derive(Component)]
    /// struct X(usize);
    /// #[derive(Component)]
    /// struct Y(usize);
    ///
    /// # let mut world = World::default();
    /// let mut entity = world.spawn((X(0), Y(0))).into_mutable();
    ///
    /// // This gives the `FilteredEntityMut` access to `&mut X` and `&mut Y`.
    /// let mut query = QueryBuilder::<FilteredEntityMut>::new(&mut world)
    ///     .data::<(&mut X, &mut Y)>()
    ///     .build();
    ///
    /// let mut filtered_entity: FilteredEntityMut = query.single_mut(&mut world).unwrap();
    ///
    /// // Get mutable access to two components at once
    /// // SAFETY: We don't take any other references to `X` from this entity
    /// let mut x = unsafe { filtered_entity.get_mut_unchecked::<X>() }.unwrap();
    /// // SAFETY: We don't take any other references to `Y` from this entity
    /// let mut y = unsafe { filtered_entity.get_mut_unchecked::<Y>() }.unwrap();
    /// *x = X(1);
    /// *y = Y(1);
    /// ```
    ///
    /// # Safety
    ///
    /// No other references to the same component may exist at the same time as the returned reference.
    ///
    /// # See also
    ///
    /// - [`get_mut`](Self::get_mut) for the safe version.
    #[inline]
    pub unsafe fn get_mut_unchecked<T: Component<Mutability = Mutable>>(
        &self,
    ) -> Option<Mut<'_, T>> {
        let id = self
            .entity
            .world()
            .components()
            .get_valid_id(TypeId::of::<T>())?;
        self.access
            .has_component_write(id)
            // SAFETY: We have permission to access the component mutable
            // and we promise to not create other references to the same component
            .then(|| unsafe { self.entity.get_mut() })
            .flatten()
    }

    /// Consumes self and gets mutable access to the component of type `T`
    /// with the world `'w` lifetime for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn into_mut<T: Component<Mutability = Mutable>>(self) -> Option<Mut<'w, T>> {
        // SAFETY:
        // - We have write access
        // - The bound `T: Component<Mutability = Mutable>` ensures the component is mutable
        unsafe { self.into_mut_assume_mutable() }
    }

    /// Consumes self and gets mutable access to the component of type `T`
    /// with the world `'w` lifetime for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Safety
    ///
    /// - `T` must be a mutable component
    #[inline]
    pub unsafe fn into_mut_assume_mutable<T: Component>(self) -> Option<Mut<'w, T>> {
        let id = self
            .entity
            .world()
            .components()
            .get_valid_id(TypeId::of::<T>())?;
        self.access
            .has_component_write(id)
            // SAFETY:
            // - We have write access
            // - Caller ensures `T` is a mutable component
            .then(|| unsafe { self.entity.get_mut_assume_mutable() })
            .flatten()
    }

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    #[inline]
    pub fn get_change_ticks<T: Component>(&self) -> Option<ComponentTicks> {
        self.as_readonly().get_change_ticks::<T>()
    }

    /// Retrieves the change ticks for the given [`ComponentId`]. This can be useful for implementing change
    /// detection in custom runtimes.
    ///
    /// **You should prefer to use the typed API [`Self::get_change_ticks`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    #[inline]
    pub fn get_change_ticks_by_id(&self, component_id: ComponentId) -> Option<ComponentTicks> {
        self.as_readonly().get_change_ticks_by_id(component_id)
    }

    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`Self::get`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`FilteredEntityMut::get`], this returns a raw pointer to the component,
    /// which is only valid while the [`FilteredEntityMut`] is alive.
    #[inline]
    pub fn get_by_id(&self, component_id: ComponentId) -> Option<Ptr<'_>> {
        self.as_readonly().get_by_id(component_id)
    }

    /// Gets a [`MutUntyped`] of the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`Self::get_mut`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`FilteredEntityMut::get_mut`], this returns a raw pointer to the component,
    /// which is only valid while the [`FilteredEntityMut`] is alive.
    #[inline]
    pub fn get_mut_by_id(&mut self, component_id: ComponentId) -> Option<MutUntyped<'_>> {
        // SAFETY: we use a mutable reference to self, so we cannot use the `FilteredEntityMut` to access
        // another component
        unsafe { self.get_mut_by_id_unchecked(component_id) }
    }

    /// Gets a [`MutUntyped`] of the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`Self::get_mut`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`FilteredEntityMut::get_mut`], this returns a raw pointer to the component,
    /// which is only valid while the [`FilteredEntityMut`] is alive.
    ///
    /// This only requires `&self`, and so may be used to get mutable access to multiple components.
    ///
    /// # Safety
    ///
    /// No other references to the same component may exist at the same time as the returned reference.
    ///
    /// # See also
    ///
    /// - [`get_mut_by_id`](Self::get_mut_by_id) for the safe version.
    #[inline]
    pub unsafe fn get_mut_by_id_unchecked(
        &self,
        component_id: ComponentId,
    ) -> Option<MutUntyped<'_>> {
        self.access
            .has_component_write(component_id)
            // SAFETY: We have permission to access the component mutable
            // and we promise to not create other references to the same component
            .then(|| unsafe { self.entity.get_mut_by_id(component_id).ok() })
            .flatten()
    }

    /// Returns the source code location from which this entity has last been spawned.
    pub fn spawned_by(&self) -> MaybeLocation {
        self.entity.spawned_by()
    }

    /// Returns the [`Tick`] at which this entity has been spawned.
    pub fn spawn_tick(&self) -> Tick {
        self.entity.spawn_tick()
    }
}

impl<'a> TryFrom<FilteredEntityMut<'a, '_>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    #[inline]
    fn try_from(entity: FilteredEntityMut<'a, '_>) -> Result<Self, Self::Error> {
        entity.into_readonly().try_into_all()
    }
}

impl<'a> TryFrom<&'a FilteredEntityMut<'_, '_>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    #[inline]
    fn try_from(entity: &'a FilteredEntityMut<'_, '_>) -> Result<Self, Self::Error> {
        entity.as_readonly().try_into_all()
    }
}

impl<'a> TryFrom<FilteredEntityMut<'a, '_>> for EntityMut<'a> {
    type Error = TryFromFilteredError;

    fn try_from(entity: FilteredEntityMut<'a, '_>) -> Result<Self, Self::Error> {
        entity.try_into_all()
    }
}

impl<'a> TryFrom<&'a mut FilteredEntityMut<'_, '_>> for EntityMut<'a> {
    type Error = TryFromFilteredError;

    #[inline]
    fn try_from(entity: &'a mut FilteredEntityMut<'_, '_>) -> Result<Self, Self::Error> {
        entity.reborrow().try_into_all()
    }
}

impl<'w, 's> From<&'w mut FilteredEntityMut<'_, 's>> for FilteredEntityMut<'w, 's> {
    #[inline]
    fn from(entity: &'w mut FilteredEntityMut<'_, 's>) -> Self {
        entity.reborrow()
    }
}

impl<'w, 's> From<FilteredEntityMut<'w, 's>> for FilteredEntityRef<'w, 's> {
    #[inline]
    fn from(entity: FilteredEntityMut<'w, 's>) -> Self {
        entity.into_readonly()
    }
}

impl<'w, 's> From<&'w FilteredEntityMut<'_, 's>> for FilteredEntityRef<'w, 's> {
    #[inline]
    fn from(entity: &'w FilteredEntityMut<'_, 's>) -> Self {
        entity.as_readonly()
    }
}

impl PartialEq for FilteredEntityMut<'_, '_> {
    fn eq(&self, other: &Self) -> bool {
        self.entity() == other.entity()
    }
}

impl Eq for FilteredEntityMut<'_, '_> {}

impl PartialOrd for FilteredEntityMut<'_, '_> {
    /// [`FilteredEntityMut`]'s comparison trait implementations match the underlying [`Entity`],
    /// and cannot discern between different worlds.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for FilteredEntityMut<'_, '_> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.entity().cmp(&other.entity())
    }
}

impl Hash for FilteredEntityMut<'_, '_> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity().hash(state);
    }
}

impl ContainsEntity for FilteredEntityMut<'_, '_> {
    fn entity(&self) -> Entity {
        self.id()
    }
}

// SAFETY: This type represents one Entity. We implement the comparison traits based on that Entity.
unsafe impl EntityEquivalent for FilteredEntityMut<'_, '_> {}

/// Error type returned by [`TryFrom`] conversions from filtered entity types
/// ([`FilteredEntityRef`]/[`FilteredEntityMut`]) to full-access entity types
/// ([`EntityRef`]/[`EntityMut`]).
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
