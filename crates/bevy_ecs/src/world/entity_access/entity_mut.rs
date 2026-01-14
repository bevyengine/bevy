use crate::{
    archetype::Archetype,
    change_detection::{ComponentTicks, MaybeLocation, Tick},
    component::{Component, ComponentId, Mutable},
    entity::{ContainsEntity, Entity, EntityEquivalent, EntityLocation},
    world::{
        error::EntityComponentError, unsafe_world_cell::UnsafeEntityCell, AccessScope, All,
        DynamicComponentFetch, EntityRef, Mut, Ref,
    },
};

use core::{
    any::TypeId,
    cmp::Ordering,
    hash::{Hash, Hasher},
};

/// Provides mutable access to a single [`Entity`] and the components allowed by
/// the [`AccessScope`] `S`. Plain `EntityMut`s have an [`AccessScope`]
/// of [`All`], providing access to all components of the entity.
///
/// Contrast with [`EntityWorldMut`], which allows adding and removing components,
/// despawning the entity, and provides mutable access to the entire world.
/// Because of this, `EntityWorldMut` cannot coexist with any other world accesses.
///
/// # [`AccessScope`]s
///
/// Access scopes describe what you can access on an `EntityMut`. The default
/// scope is [`All`], which provides access to all components of the entity.
/// Other scopes, such as [`Filtered`] and [`Except`], can restrict access to
/// only a subset of components.
///
/// See the documentation of [`AccessScope`] for more details.
///
/// # Examples
///
/// Disjoint mutable access.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)] pub struct A;
/// fn disjoint_system(
///     query1: Query<EntityMut, With<A>>,
///     query2: Query<EntityMut, Without<A>>,
/// ) {
///     // ...
/// }
/// # bevy_ecs::system::assert_is_system(disjoint_system);
/// ```
///
/// [`EntityWorldMut`]: crate::world::EntityWorldMut
/// [`Filtered`]: crate::world::Filtered
/// [`Except`]: crate::world::Except
pub struct EntityMut<'w, S: AccessScope = All> {
    pub(super) cell: UnsafeEntityCell<'w>,
    scope: S,
}

impl<'w, S: AccessScope> EntityMut<'w, S> {
    /// # Safety
    ///
    /// Caller must ensure `scope` does not exceed the read or write permissions
    /// of `cell` in a way that would violate Rust's aliasing rules, including
    /// simultaneous access of `cell` via another `EntityMut`, `EntityRef`, or
    /// any other means.
    #[inline]
    pub(crate) unsafe fn new(cell: UnsafeEntityCell<'w>, scope: S) -> Self {
        Self { cell, scope }
    }

    /// Returns a new instance with a shorter lifetime.
    /// This is useful if you have `&mut EntityMut`, but you need `EntityMut`.
    #[inline]
    pub fn reborrow(&mut self) -> EntityMut<'_, S::Borrow<'_>> {
        // SAFETY: We have exclusive access to the entire entity and its components.
        unsafe { EntityMut::new(self.cell, self.scope.reborrow()) }
    }

    /// Consumes `self` and returns a [`EntityRef`] with the same access
    /// permissions.
    #[inline]
    pub fn into_readonly(self) -> EntityRef<'w, S> {
        // SAFETY:
        // - Read permissions of `entity.scope` are preserved.
        // - Consuming `entity` ensures there are no mutable accesses.
        unsafe { EntityRef::new(self.cell, self.scope) }
    }

    /// Borrows `self` and returns a [`EntityRef`] with the same access
    /// permissions.
    #[inline]
    pub fn as_readonly(&self) -> EntityRef<'_, S::Borrow<'_>> {
        // SAFETY:
        // - Read permissions of `&entity.scope` are preserved.
        // - `&entity` ensures there are no mutable accesses.
        unsafe { EntityRef::new(self.cell, self.scope.reborrow()) }
    }

    /// Get access to the underlying [`UnsafeEntityCell`].
    #[inline]
    pub fn as_unsafe_entity_cell(&mut self) -> UnsafeEntityCell<'_> {
        self.cell
    }

    /// Returns a reference to the current [`AccessScope`].
    #[inline]
    pub fn scope(&self) -> &S {
        &self.scope
    }

    /// Consumes self and returns the current [`AccessScope`].
    #[inline]
    pub fn into_scope(self) -> S {
        self.scope
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.cell.id()
    }

    /// Gets metadata indicating the location where the current entity is stored.
    #[inline]
    pub fn location(&self) -> EntityLocation {
        self.cell.location()
    }

    /// Returns the archetype that the current entity belongs to.
    #[inline]
    pub fn archetype(&self) -> &Archetype {
        self.cell.archetype()
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
        self.cell.contains_id(component_id)
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
        self.cell.contains_type_id(type_id)
    }

    /// Gets access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'_ T> {
        self.as_readonly().get()
    }

    /// Consumes `self` and gets access to the component of type `T` with the
    /// world `'w` lifetime for the current entity.
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn into_borrow<T: Component>(self) -> Option<&'w T> {
        self.into_readonly().get()
    }

    /// Gets access to the component of type `T` for the current entity,
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_ref<T: Component>(&self) -> Option<Ref<'_, T>> {
        self.as_readonly().get_ref()
    }

    /// Consumes `self` and gets access to the component of type `T` with world
    /// `'w` lifetime for the current entity, including change detection information
    /// as a [`Ref<'w>`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn into_ref<T: Component>(self) -> Option<Ref<'w, T>> {
        self.into_readonly().get_ref()
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_mut<T: Component<Mutability = Mutable>>(&mut self) -> Option<Mut<'_, T>> {
        self.reborrow().into_mut()
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Safety
    ///
    /// - `T` must be a mutable component
    #[inline]
    pub unsafe fn get_mut_assume_mutable<T: Component>(&mut self) -> Option<Mut<'_, T>> {
        self.reborrow().into_mut_assume_mutable()
    }

    /// Consumes self and gets mutable access to the component of type `T`
    /// with the world `'w` lifetime for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn into_mut<T: Component<Mutability = Mutable>>(self) -> Option<Mut<'w, T>> {
        // SAFETY:
        // - `self` was constructed with a `scope` that doesn't violate aliasing
        //   rules for `cell`.
        // - Consuming `self` implies exclusive access to components in `scope`.
        unsafe { self.cell.get_mut(&self.scope) }
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Safety
    ///
    /// - `T` must be a mutable component
    #[inline]
    pub unsafe fn into_mut_assume_mutable<T: Component>(self) -> Option<Mut<'w, T>> {
        // SAFETY:
        // - `self` was constructed with a `scope` that doesn't violate aliasing
        //   rules for `cell`.
        // - Consuming `self` implies exclusive access to components in `scope`.
        // - Caller ensures `T` is a mutable component.
        unsafe { self.cell.get_mut_assume_mutable(&self.scope) }
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
    /// **You should prefer to use the typed API [`EntityWorldMut::get_change_ticks`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// [`EntityWorldMut::get_change_ticks`]: crate::world::EntityWorldMut::get_change_ticks
    #[inline]
    pub fn get_change_ticks_by_id(&self, component_id: ComponentId) -> Option<ComponentTicks> {
        self.as_readonly().get_change_ticks_by_id(component_id)
    }

    /// Returns untyped read-only reference(s) to component(s) for the
    /// current entity, based on the given [`ComponentId`]s.
    ///
    /// **You should prefer to use the typed API [`EntityMut::get`] where
    /// possible and only use this in cases where the actual component types
    /// are not known at compile time.**
    ///
    /// Unlike [`EntityMut::get`], this returns untyped reference(s) to
    /// component(s), and it's the job of the caller to ensure the correct
    /// type(s) are dereferenced (if necessary).
    ///
    /// # Errors
    ///
    /// Returns [`EntityComponentError::MissingComponent`] if the entity does
    /// not have a component.
    ///
    /// # Examples
    ///
    /// For examples on how to use this method, see [`EntityRef::get_by_id`].
    #[inline]
    pub fn get_by_id<F: DynamicComponentFetch>(
        &self,
        component_ids: F,
    ) -> Result<F::Ref<'_>, EntityComponentError> {
        self.as_readonly().get_by_id(component_ids)
    }

    /// Consumes `self` and returns untyped read-only reference(s) to
    /// component(s) with lifetime `'w` for the current entity, based on the
    /// given [`ComponentId`]s.
    ///
    /// **You should prefer to use the typed API [`EntityMut::into_borrow`]
    /// where possible and only use this in cases where the actual component
    /// types are not known at compile time.**
    ///
    /// Unlike [`EntityMut::into_borrow`], this returns untyped reference(s) to
    /// component(s), and it's the job of the caller to ensure the correct
    /// type(s) are dereferenced (if necessary).
    ///
    /// # Errors
    ///
    /// Returns [`EntityComponentError::MissingComponent`] if the entity does
    /// not have a component.
    ///
    /// # Examples
    ///
    /// For examples on how to use this method, see [`EntityRef::get_by_id`].
    #[inline]
    pub fn into_borrow_by_id<F: DynamicComponentFetch>(
        self,
        component_ids: F,
    ) -> Result<F::Ref<'w>, EntityComponentError> {
        self.into_readonly().get_by_id(component_ids)
    }

    /// Returns untyped mutable reference(s) to component(s) for
    /// the current entity, based on the given [`ComponentId`]s.
    ///
    /// **You should prefer to use the typed API [`EntityMut::get_mut`] where
    /// possible and only use this in cases where the actual component types
    /// are not known at compile time.**
    ///
    /// Unlike [`EntityMut::get_mut`], this returns untyped reference(s) to
    /// component(s), and it's the job of the caller to ensure the correct
    /// type(s) are dereferenced (if necessary).
    ///
    /// # Errors
    ///
    /// - Returns [`EntityComponentError::MissingComponent`] if the entity does
    ///   not have a component.
    /// - Returns [`EntityComponentError::AliasedMutability`] if a component
    ///   is requested multiple times.
    ///
    /// # Examples
    ///
    /// ## Single [`ComponentId`]
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component, PartialEq, Debug)]
    /// # pub struct Foo(i32);
    /// # let mut world = World::new();
    /// let entity = world.spawn(Foo(42)).id();
    ///
    /// // Grab the component ID for `Foo` in whatever way you like.
    /// let component_id = world.register_component::<Foo>();
    ///
    /// // Then, get the component by ID.
    /// let mut entity_mut = world.entity_mut(entity);
    /// let mut ptr = entity_mut.get_mut_by_id(component_id)
    /// #   .unwrap();
    /// # assert_eq!(unsafe { ptr.as_mut().deref_mut::<Foo>() }, &mut Foo(42));
    /// ```
    ///
    /// ## Array of [`ComponentId`]s
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// # #[derive(Component, PartialEq, Debug)]
    /// # pub struct X(i32);
    /// # #[derive(Component, PartialEq, Debug)]
    /// # pub struct Y(i32);
    /// # let mut world = World::new();
    /// let entity = world.spawn((X(42), Y(10))).id();
    ///
    /// // Grab the component IDs for `X` and `Y` in whatever way you like.
    /// let x_id = world.register_component::<X>();
    /// let y_id = world.register_component::<Y>();
    ///
    /// // Then, get the components by ID. You'll receive a same-sized array.
    /// let mut entity_mut = world.entity_mut(entity);
    /// let Ok([mut x_ptr, mut y_ptr]) = entity_mut.get_mut_by_id([x_id, y_id]) else {
    ///     // Up to you to handle if a component is missing from the entity.
    /// #   unreachable!();
    /// };
    /// # assert_eq!((unsafe { x_ptr.as_mut().deref_mut::<X>() }, unsafe { y_ptr.as_mut().deref_mut::<Y>() }), (&mut X(42), &mut Y(10)));
    /// ```
    ///
    /// ## Slice of [`ComponentId`]s
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, component::ComponentId, change_detection::MutUntyped};
    /// #
    /// # #[derive(Component, PartialEq, Debug)]
    /// # pub struct X(i32);
    /// # #[derive(Component, PartialEq, Debug)]
    /// # pub struct Y(i32);
    /// # let mut world = World::new();
    /// let entity = world.spawn((X(42), Y(10))).id();
    ///
    /// // Grab the component IDs for `X` and `Y` in whatever way you like.
    /// let x_id = world.register_component::<X>();
    /// let y_id = world.register_component::<Y>();
    ///
    /// // Then, get the components by ID. You'll receive a vec of ptrs.
    /// let mut entity_mut = world.entity_mut(entity);
    /// let ptrs = entity_mut.get_mut_by_id(&[x_id, y_id] as &[ComponentId])
    /// #   .unwrap();
    /// # let [mut x_ptr, mut y_ptr]: [MutUntyped; 2] = ptrs.try_into().unwrap();
    /// # assert_eq!((unsafe { x_ptr.as_mut().deref_mut::<X>() }, unsafe { y_ptr.as_mut().deref_mut::<Y>() }), (&mut X(42), &mut Y(10)));
    /// ```
    ///
    /// ## `HashSet` of [`ComponentId`]s
    ///
    /// ```
    /// # use bevy_platform::collections::HashSet;
    /// # use bevy_ecs::{prelude::*, component::ComponentId};
    /// #
    /// # #[derive(Component, PartialEq, Debug)]
    /// # pub struct X(i32);
    /// # #[derive(Component, PartialEq, Debug)]
    /// # pub struct Y(i32);
    /// # let mut world = World::new();
    /// let entity = world.spawn((X(42), Y(10))).id();
    ///
    /// // Grab the component IDs for `X` and `Y` in whatever way you like.
    /// let x_id = world.register_component::<X>();
    /// let y_id = world.register_component::<Y>();
    ///
    /// // Then, get the components by ID. You'll receive a `HashMap` of ptrs.
    /// let mut entity_mut = world.entity_mut(entity);
    /// let mut ptrs = entity_mut.get_mut_by_id(&HashSet::from_iter([x_id, y_id]))
    /// #   .unwrap();
    /// # let [Some(mut x_ptr), Some(mut y_ptr)] = ptrs.get_many_mut([&x_id, &y_id]) else { unreachable!() };
    /// # assert_eq!((unsafe { x_ptr.as_mut().deref_mut::<X>() }, unsafe { y_ptr.as_mut().deref_mut::<Y>() }), (&mut X(42), &mut Y(10)));
    /// ```
    #[inline]
    pub fn get_mut_by_id<F: DynamicComponentFetch>(
        &mut self,
        component_ids: F,
    ) -> Result<F::Mut<'_>, EntityComponentError> {
        self.reborrow().into_mut_by_id(component_ids)
    }

    /// Returns untyped mutable reference(s) to component(s) for
    /// the current entity, based on the given [`ComponentId`]s.
    /// Assumes the given [`ComponentId`]s refer to mutable components.
    ///
    /// **You should prefer to use the typed API [`EntityMut::get_mut_assume_mutable`] where
    /// possible and only use this in cases where the actual component types
    /// are not known at compile time.**
    ///
    /// Unlike [`EntityMut::get_mut_assume_mutable`], this returns untyped reference(s) to
    /// component(s), and it's the job of the caller to ensure the correct
    /// type(s) are dereferenced (if necessary).
    ///
    /// # Errors
    ///
    /// - Returns [`EntityComponentError::MissingComponent`] if the entity does
    ///   not have a component.
    /// - Returns [`EntityComponentError::AliasedMutability`] if a component
    ///   is requested multiple times.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the provided [`ComponentId`]s must refer to mutable components.
    #[inline]
    pub unsafe fn get_mut_assume_mutable_by_id<F: DynamicComponentFetch>(
        &mut self,
        component_ids: F,
    ) -> Result<F::Mut<'_>, EntityComponentError> {
        self.reborrow().into_mut_assume_mutable_by_id(component_ids)
    }

    /// Returns untyped mutable reference to component for
    /// the current entity, based on the given [`ComponentId`].
    ///
    /// Unlike [`EntityMut::get_mut_by_id`], this method borrows &self instead of
    /// &mut self, allowing the caller to access multiple components simultaneously.
    ///
    /// # Errors
    ///
    /// - Returns [`EntityComponentError::MissingComponent`] if the entity does
    ///   not have a component.
    /// - Returns [`EntityComponentError::AliasedMutability`] if a component
    ///   is requested multiple times.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeEntityCell`] has permission to access the component mutably
    /// - no other references to the component exist at the same time
    #[inline]
    pub unsafe fn get_mut_by_id_unchecked<F: DynamicComponentFetch>(
        &self,
        component_ids: F,
    ) -> Result<F::Mut<'_>, EntityComponentError> {
        // SAFETY:
        // - `self` was constructed with a `scope` that doesn't violate aliasing
        //   rules for `cell`.
        // - Caller ensures exclusive access to components in `scope` for
        //   duration of returned value.
        unsafe { component_ids.fetch_mut(self.cell, &self.scope) }
    }

    /// Returns untyped mutable reference to component for
    /// the current entity, based on the given [`ComponentId`].
    /// Assumes the given [`ComponentId`]s refer to mutable components.
    ///
    /// Unlike [`EntityMut::get_mut_assume_mutable_by_id`], this method borrows &self instead of
    /// &mut self, allowing the caller to access multiple components simultaneously.
    ///
    /// # Errors
    ///
    /// - Returns [`EntityComponentError::MissingComponent`] if the entity does
    ///   not have a component.
    /// - Returns [`EntityComponentError::AliasedMutability`] if a component
    ///   is requested multiple times.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeEntityCell`] has permission to access the component mutably
    /// - no other references to the component exist at the same time
    /// - the provided [`ComponentId`]s must refer to mutable components.
    #[inline]
    pub unsafe fn get_mut_assume_mutable_by_id_unchecked<F: DynamicComponentFetch>(
        &self,
        component_ids: F,
    ) -> Result<F::Mut<'_>, EntityComponentError> {
        // SAFETY:
        // - `self` was constructed with a `scope` that doesn't violate aliasing
        //   rules for `cell`.
        // - Caller ensures exclusive access to components in `scope` for
        //   duration of returned value.
        // - Caller ensures provided `ComponentId`s refer to mutable components.
        unsafe { component_ids.fetch_mut_assume_mutable(self.cell, &self.scope) }
    }

    /// Consumes `self` and returns untyped mutable reference(s)
    /// to component(s) with lifetime `'w` for the current entity, based on the
    /// given [`ComponentId`]s.
    ///
    /// **You should prefer to use the typed API [`EntityMut::into_mut`] where
    /// possible and only use this in cases where the actual component types
    /// are not known at compile time.**
    ///
    /// Unlike [`EntityMut::into_mut`], this returns untyped reference(s) to
    /// component(s), and it's the job of the caller to ensure the correct
    /// type(s) are dereferenced (if necessary).
    ///
    /// # Errors
    ///
    /// - Returns [`EntityComponentError::MissingComponent`] if the entity does
    ///   not have a component.
    /// - Returns [`EntityComponentError::AliasedMutability`] if a component
    ///   is requested multiple times.
    ///
    /// # Examples
    ///
    /// For examples on how to use this method, see [`EntityMut::get_mut_by_id`].
    #[inline]
    pub fn into_mut_by_id<F: DynamicComponentFetch>(
        self,
        component_ids: F,
    ) -> Result<F::Mut<'w>, EntityComponentError> {
        // SAFETY:
        // - `self` was constructed with a `scope` that doesn't violate aliasing
        //   rules for `cell`.
        // - Consuming `self` implies exclusive access to components in `scope`.
        unsafe { component_ids.fetch_mut(self.cell, &self.scope) }
    }

    /// Consumes `self` and returns untyped mutable reference(s)
    /// to component(s) with lifetime `'w` for the current entity, based on the
    /// given [`ComponentId`]s.
    /// Assumes the given [`ComponentId`]s refer to mutable components.
    ///
    /// **You should prefer to use the typed API [`EntityMut::into_mut_assume_mutable`] where
    /// possible and only use this in cases where the actual component types
    /// are not known at compile time.**
    ///
    /// Unlike [`EntityMut::into_mut_assume_mutable`], this returns untyped reference(s) to
    /// component(s), and it's the job of the caller to ensure the correct
    /// type(s) are dereferenced (if necessary).
    ///
    /// # Errors
    ///
    /// - Returns [`EntityComponentError::MissingComponent`] if the entity does
    ///   not have a component.
    /// - Returns [`EntityComponentError::AliasedMutability`] if a component
    ///   is requested multiple times.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the provided [`ComponentId`]s must refer to mutable components.
    #[inline]
    pub unsafe fn into_mut_assume_mutable_by_id<F: DynamicComponentFetch>(
        self,
        component_ids: F,
    ) -> Result<F::Mut<'w>, EntityComponentError> {
        // SAFETY:
        // - `self` was constructed with a `scope` that doesn't violate aliasing
        //   rules for `cell`.
        // - Consuming `self` implies exclusive access to components in `scope`.
        // - Caller ensures provided `ComponentId`s refer to mutable components.
        unsafe { component_ids.fetch_mut_assume_mutable(self.cell, &self.scope) }
    }

    /// Returns the source code location from which this entity has been spawned.
    pub fn spawned_by(&self) -> MaybeLocation {
        self.cell.spawned_by()
    }

    /// Returns the [`Tick`] at which this entity has been spawned.
    pub fn spawn_tick(&self) -> Tick {
        self.cell.spawn_tick()
    }
}

impl<'w, S: AccessScope> From<EntityMut<'w, S>> for EntityRef<'w, S> {
    #[inline]
    fn from(entity: EntityMut<'w, S>) -> Self {
        entity.into_readonly()
    }
}

impl<'w, S: AccessScope> From<&'w EntityMut<'_, S>> for EntityRef<'w, S::Borrow<'w>> {
    #[inline]
    fn from(entity: &'w EntityMut<'_, S>) -> Self {
        entity.as_readonly()
    }
}

impl<'w, S: AccessScope> From<&'w mut EntityMut<'_, S>> for EntityMut<'w, S::Borrow<'w>> {
    #[inline]
    fn from(entity: &'w mut EntityMut<'_, S>) -> Self {
        entity.reborrow()
    }
}

impl<S: AccessScope> PartialEq for EntityMut<'_, S> {
    fn eq(&self, other: &Self) -> bool {
        self.entity() == other.entity()
    }
}

impl<S: AccessScope> Eq for EntityMut<'_, S> {}

impl<S: AccessScope> PartialOrd for EntityMut<'_, S> {
    /// [`EntityMut`]'s comparison trait implementations match the underlying [`Entity`],
    /// and cannot discern between different worlds.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: AccessScope> Ord for EntityMut<'_, S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.entity().cmp(&other.entity())
    }
}

impl<S: AccessScope> Hash for EntityMut<'_, S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity().hash(state);
    }
}

impl<S: AccessScope> ContainsEntity for EntityMut<'_, S> {
    fn entity(&self) -> Entity {
        self.id()
    }
}

// SAFETY: This type represents one Entity. We implement the comparison traits based on that Entity.
unsafe impl<S: AccessScope> EntityEquivalent for EntityMut<'_, S> {}
