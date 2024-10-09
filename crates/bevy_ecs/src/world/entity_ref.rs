use crate::{
    archetype::{Archetype, ArchetypeId, Archetypes},
    bundle::{Bundle, BundleId, BundleInfo, BundleInserter, DynamicBundle, InsertMode},
    change_detection::MutUntyped,
    component::{Component, ComponentId, ComponentTicks, Components, StorageType},
    entity::{Entities, Entity, EntityLocation},
    event::Event,
    observer::{Observer, Observers},
    query::{Access, ReadOnlyQueryData},
    removal_detection::RemovedComponentEvents,
    storage::Storages,
    system::IntoObserverSystem,
    world::{error::EntityComponentError, DeferredWorld, Mut, World},
};
use bevy_ptr::{OwningPtr, Ptr};
use bevy_utils::{HashMap, HashSet};
use core::{any::TypeId, marker::PhantomData, mem::MaybeUninit};
use derive_more::derive::{Display, Error};

use super::{unsafe_world_cell::UnsafeEntityCell, Ref, ON_REMOVE, ON_REPLACE};

/// A read-only reference to a particular [`Entity`] and all of its components.
///
/// # Examples
///
/// Read-only access disjoint with mutable access.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)] pub struct A;
/// # #[derive(Component)] pub struct B;
/// fn disjoint_system(
///     query1: Query<&mut A>,
///     query2: Query<EntityRef, Without<A>>,
/// ) {
///     // ...
/// }
/// # bevy_ecs::system::assert_is_system(disjoint_system);
/// ```
#[derive(Copy, Clone)]
pub struct EntityRef<'w>(UnsafeEntityCell<'w>);

impl<'w> EntityRef<'w> {
    /// # Safety
    /// - `cell` must have permission to read every component of the entity.
    /// - No mutable accesses to any of the entity's components may exist
    ///   at the same time as the returned [`EntityRef`].
    #[inline]
    pub(crate) unsafe fn new(cell: UnsafeEntityCell<'w>) -> Self {
        Self(cell)
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.0.id()
    }

    /// Gets metadata indicating the location where the current entity is stored.
    #[inline]
    pub fn location(&self) -> EntityLocation {
        self.0.location()
    }

    /// Returns the archetype that the current entity belongs to.
    #[inline]
    pub fn archetype(&self) -> &Archetype {
        self.0.archetype()
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
    ///     [`Self::contains_type_id`].
    #[inline]
    pub fn contains_id(&self, component_id: ComponentId) -> bool {
        self.0.contains_id(component_id)
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
        self.0.contains_type_id(type_id)
    }

    /// Gets access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'w T> {
        // SAFETY: We have read-only access to all components of this entity.
        unsafe { self.0.get::<T>() }
    }

    /// Gets access to the component of type `T` for the current entity,
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_ref<T: Component>(&self) -> Option<Ref<'w, T>> {
        // SAFETY: We have read-only access to all components of this entity.
        unsafe { self.0.get_ref::<T>() }
    }

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    #[inline]
    pub fn get_change_ticks<T: Component>(&self) -> Option<ComponentTicks> {
        // SAFETY: We have read-only access to all components of this entity.
        unsafe { self.0.get_change_ticks::<T>() }
    }

    /// Retrieves the change ticks for the given [`ComponentId`]. This can be useful for implementing change
    /// detection in custom runtimes.
    ///
    /// **You should prefer to use the typed API [`EntityRef::get_change_ticks`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    #[inline]
    pub fn get_change_ticks_by_id(&self, component_id: ComponentId) -> Option<ComponentTicks> {
        // SAFETY: We have read-only access to all components of this entity.
        unsafe { self.0.get_change_ticks_by_id(component_id) }
    }

    /// Returns [untyped read-only reference(s)](Ptr) to component(s) for the
    /// current entity, based on the given [`ComponentId`]s.
    ///
    /// **You should prefer to use the typed API [`EntityRef::get`] where
    /// possible and only use this in cases where the actual component types
    /// are not known at compile time.**
    ///
    /// Unlike [`EntityRef::get`], this returns untyped reference(s) to
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
    /// let ptr = world.entity(entity).get_by_id(component_id);
    /// # assert_eq!(unsafe { ptr.unwrap().deref::<Foo>() }, &Foo(42));
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
    /// let Ok([x_ptr, y_ptr]) = world.entity(entity).get_by_id([x_id, y_id]) else {
    ///     // Up to you to handle if a component is missing from the entity.
    /// #   unreachable!();
    /// };
    /// # assert_eq!((unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() }), (&X(42), &Y(10)));
    /// ```
    ///
    /// ## Slice of [`ComponentId`]s
    ///
    /// ```
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
    /// // Then, get the components by ID. You'll receive a vec of ptrs.
    /// let ptrs = world.entity(entity).get_by_id(&[x_id, y_id] as &[ComponentId]);
    /// # let ptrs = ptrs.unwrap();
    /// # assert_eq!((unsafe { ptrs[0].deref::<X>() }, unsafe { ptrs[1].deref::<Y>() }), (&X(42), &Y(10)));
    /// ```
    ///
    /// ## [`HashSet`] of [`ComponentId`]s
    ///
    /// ```
    /// # use bevy_utils::HashSet;
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
    /// // Then, get the components by ID. You'll receive a vec of ptrs.
    /// let ptrs = world.entity(entity).get_by_id(&HashSet::from_iter([x_id, y_id]));
    /// # let ptrs = ptrs.unwrap();
    /// # assert_eq!((unsafe { ptrs[&x_id].deref::<X>() }, unsafe { ptrs[&y_id].deref::<Y>() }), (&X(42), &Y(10)));
    /// ```
    #[inline]
    pub fn get_by_id<F: DynamicComponentFetch>(
        &self,
        component_ids: F,
    ) -> Result<F::Ref<'w>, EntityComponentError> {
        // SAFETY: We have read-only access to all components of this entity.
        unsafe { component_ids.fetch_ref(self.0) }
    }

    /// Returns read-only components for the current entity that match the query `Q`.
    ///
    /// # Panics
    ///
    /// If the entity does not have the components required by the query `Q`.
    pub fn components<Q: ReadOnlyQueryData>(&self) -> Q::Item<'w> {
        self.get_components::<Q>().expect(QUERY_MISMATCH_ERROR)
    }

    /// Returns read-only components for the current entity that match the query `Q`,
    /// or `None` if the entity does not have the components required by the query `Q`.
    pub fn get_components<Q: ReadOnlyQueryData>(&self) -> Option<Q::Item<'w>> {
        // SAFETY: We have read-only access to all components of this entity.
        unsafe { self.0.get_components::<Q>() }
    }
}

impl<'w> From<EntityWorldMut<'w>> for EntityRef<'w> {
    fn from(entity_mut: EntityWorldMut<'w>) -> EntityRef<'w> {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe { EntityRef::new(entity_mut.into_unsafe_entity_cell()) }
    }
}

impl<'a> From<&'a EntityWorldMut<'_>> for EntityRef<'a> {
    fn from(value: &'a EntityWorldMut<'_>) -> Self {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        // - `&value` ensures no mutable accesses are active.
        unsafe { EntityRef::new(value.as_unsafe_entity_cell_readonly()) }
    }
}

impl<'w> From<EntityMut<'w>> for EntityRef<'w> {
    fn from(value: EntityMut<'w>) -> Self {
        // SAFETY:
        // - `EntityMut` guarantees exclusive access to all of the entity's components.
        unsafe { EntityRef::new(value.0) }
    }
}

impl<'a> From<&'a EntityMut<'_>> for EntityRef<'a> {
    fn from(value: &'a EntityMut<'_>) -> Self {
        // SAFETY:
        // - `EntityMut` guarantees exclusive access to all of the entity's components.
        // - `&value` ensures there are no mutable accesses.
        unsafe { EntityRef::new(value.0) }
    }
}

impl<'a> TryFrom<FilteredEntityRef<'a>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    fn try_from(value: FilteredEntityRef<'a>) -> Result<Self, Self::Error> {
        if !value.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else {
            // SAFETY: check above guarantees read-only access to all components of the entity.
            Ok(unsafe { EntityRef::new(value.entity) })
        }
    }
}

impl<'a> TryFrom<&'a FilteredEntityRef<'_>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    fn try_from(value: &'a FilteredEntityRef<'_>) -> Result<Self, Self::Error> {
        if !value.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else {
            // SAFETY: check above guarantees read-only access to all components of the entity.
            Ok(unsafe { EntityRef::new(value.entity) })
        }
    }
}

impl<'a> TryFrom<FilteredEntityMut<'a>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    fn try_from(value: FilteredEntityMut<'a>) -> Result<Self, Self::Error> {
        if !value.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else {
            // SAFETY: check above guarantees read-only access to all components of the entity.
            Ok(unsafe { EntityRef::new(value.entity) })
        }
    }
}

impl<'a> TryFrom<&'a FilteredEntityMut<'_>> for EntityRef<'a> {
    type Error = TryFromFilteredError;

    fn try_from(value: &'a FilteredEntityMut<'_>) -> Result<Self, Self::Error> {
        if !value.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else {
            // SAFETY: check above guarantees read-only access to all components of the entity.
            Ok(unsafe { EntityRef::new(value.entity) })
        }
    }
}

/// Provides mutable access to a single entity and all of its components.
///
/// Contrast with [`EntityWorldMut`], which allows adding and removing components,
/// despawning the entity, and provides mutable access to the entire world.
/// Because of this, `EntityWorldMut` cannot coexist with any other world accesses.
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
pub struct EntityMut<'w>(UnsafeEntityCell<'w>);

impl<'w> EntityMut<'w> {
    /// # Safety
    /// - `cell` must have permission to mutate every component of the entity.
    /// - No accesses to any of the entity's components may exist
    ///   at the same time as the returned [`EntityMut`].
    pub(crate) unsafe fn new(cell: UnsafeEntityCell<'w>) -> Self {
        Self(cell)
    }

    /// Returns a new instance with a shorter lifetime.
    /// This is useful if you have `&mut EntityMut`, but you need `EntityMut`.
    pub fn reborrow(&mut self) -> EntityMut<'_> {
        // SAFETY: We have exclusive access to the entire entity and its components.
        unsafe { Self::new(self.0) }
    }

    /// Gets read-only access to all of the entity's components.
    pub fn as_readonly(&self) -> EntityRef<'_> {
        EntityRef::from(self)
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.0.id()
    }

    /// Gets metadata indicating the location where the current entity is stored.
    #[inline]
    pub fn location(&self) -> EntityLocation {
        self.0.location()
    }

    /// Returns the archetype that the current entity belongs to.
    #[inline]
    pub fn archetype(&self) -> &Archetype {
        self.0.archetype()
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
    ///     [`Self::contains_type_id`].
    #[inline]
    pub fn contains_id(&self, component_id: ComponentId) -> bool {
        self.0.contains_id(component_id)
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
        self.0.contains_type_id(type_id)
    }

    /// Gets access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'_ T> {
        self.as_readonly().get()
    }

    /// Returns read-only components for the current entity that match the query `Q`.
    ///
    /// # Panics
    ///
    /// If the entity does not have the components required by the query `Q`.
    pub fn components<Q: ReadOnlyQueryData>(&self) -> Q::Item<'_> {
        self.get_components::<Q>().expect(QUERY_MISMATCH_ERROR)
    }

    /// Returns read-only components for the current entity that match the query `Q`,
    /// or `None` if the entity does not have the components required by the query `Q`.
    pub fn get_components<Q: ReadOnlyQueryData>(&self) -> Option<Q::Item<'_>> {
        // SAFETY: We have read-only access to all components of this entity.
        unsafe { self.0.get_components::<Q>() }
    }

    /// Consumes `self` and gets access to the component of type `T` with the
    /// world `'w` lifetime for the current entity.
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn into_borrow<T: Component>(self) -> Option<&'w T> {
        // SAFETY: consuming `self` implies exclusive access
        unsafe { self.0.get() }
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
        // SAFETY: consuming `self` implies exclusive access
        unsafe { self.0.get_ref() }
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_mut<T: Component>(&mut self) -> Option<Mut<'_, T>> {
        // SAFETY: &mut self implies exclusive access for duration of returned value
        unsafe { self.0.get_mut() }
    }

    /// Consumes self and gets mutable access to the component of type `T`
    /// with the world `'w` lifetime for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn into_mut<T: Component>(self) -> Option<Mut<'w, T>> {
        // SAFETY: consuming `self` implies exclusive access
        unsafe { self.0.get_mut() }
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
    #[inline]
    pub fn get_change_ticks_by_id(&self, component_id: ComponentId) -> Option<ComponentTicks> {
        self.as_readonly().get_change_ticks_by_id(component_id)
    }

    /// Returns [untyped read-only reference(s)](Ptr) to component(s) for the
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

    /// Consumes `self` and returns [untyped read-only reference(s)](Ptr) to
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
        // SAFETY:
        // - We have read-only access to all components of this entity.
        // - consuming `self` ensures that no references exist to this entity's components.
        unsafe { component_ids.fetch_ref(self.0) }
    }

    /// Returns [untyped mutable reference(s)](MutUntyped) to component(s) for
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
    /// ## [`HashSet`] of [`ComponentId`]s
    ///
    /// ```
    /// # use bevy_utils::HashSet;
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
    /// # let [mut x_ptr, mut y_ptr] = ptrs.get_many_mut([&x_id, &y_id]).unwrap();
    /// # assert_eq!((unsafe { x_ptr.as_mut().deref_mut::<X>() }, unsafe { y_ptr.as_mut().deref_mut::<Y>() }), (&mut X(42), &mut Y(10)));
    /// ```
    #[inline]
    pub fn get_mut_by_id<F: DynamicComponentFetch>(
        &mut self,
        component_ids: F,
    ) -> Result<F::Mut<'_>, EntityComponentError> {
        // SAFETY:
        // - `&mut self` ensures that no references exist to this entity's components.
        // - We have exclusive access to all components of this entity.
        unsafe { component_ids.fetch_mut(self.0) }
    }

    /// Consumes `self` and returns [untyped mutable reference(s)](MutUntyped)
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
        // - consuming `self` ensures that no references exist to this entity's components.
        // - We have exclusive access to all components of this entity.
        unsafe { component_ids.fetch_mut(self.0) }
    }
}

impl<'w> From<&'w mut EntityMut<'_>> for EntityMut<'w> {
    fn from(value: &'w mut EntityMut<'_>) -> Self {
        value.reborrow()
    }
}

impl<'w> From<EntityWorldMut<'w>> for EntityMut<'w> {
    fn from(value: EntityWorldMut<'w>) -> Self {
        // SAFETY: `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe { EntityMut::new(value.into_unsafe_entity_cell()) }
    }
}

impl<'a> From<&'a mut EntityWorldMut<'_>> for EntityMut<'a> {
    fn from(value: &'a mut EntityWorldMut<'_>) -> Self {
        // SAFETY: `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe { EntityMut::new(value.as_unsafe_entity_cell()) }
    }
}

impl<'a> TryFrom<FilteredEntityMut<'a>> for EntityMut<'a> {
    type Error = TryFromFilteredError;

    fn try_from(value: FilteredEntityMut<'a>) -> Result<Self, Self::Error> {
        if !value.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else if !value.access.has_write_all() {
            Err(TryFromFilteredError::MissingWriteAllAccess)
        } else {
            // SAFETY: check above guarantees exclusive access to all components of the entity.
            Ok(unsafe { EntityMut::new(value.entity) })
        }
    }
}

impl<'a> TryFrom<&'a mut FilteredEntityMut<'_>> for EntityMut<'a> {
    type Error = TryFromFilteredError;

    fn try_from(value: &'a mut FilteredEntityMut<'_>) -> Result<Self, Self::Error> {
        if !value.access.has_read_all() {
            Err(TryFromFilteredError::MissingReadAllAccess)
        } else if !value.access.has_write_all() {
            Err(TryFromFilteredError::MissingWriteAllAccess)
        } else {
            // SAFETY: check above guarantees exclusive access to all components of the entity.
            Ok(unsafe { EntityMut::new(value.entity) })
        }
    }
}

/// A mutable reference to a particular [`Entity`], and the entire world.
///
/// This is essentially a performance-optimized `(Entity, &mut World)` tuple,
/// which caches the [`EntityLocation`] to reduce duplicate lookups.
///
/// Since this type provides mutable access to the entire world, only one
/// [`EntityWorldMut`] can exist at a time for a given world.
///
/// See also [`EntityMut`], which allows disjoint mutable access to multiple
/// entities at once.  Unlike `EntityMut`, this type allows adding and
/// removing components, and despawning the entity.
pub struct EntityWorldMut<'w> {
    world: &'w mut World,
    entity: Entity,
    location: EntityLocation,
}

impl<'w> EntityWorldMut<'w> {
    fn as_unsafe_entity_cell_readonly(&self) -> UnsafeEntityCell<'_> {
        UnsafeEntityCell::new(
            self.world.as_unsafe_world_cell_readonly(),
            self.entity,
            self.location,
        )
    }
    fn as_unsafe_entity_cell(&mut self) -> UnsafeEntityCell<'_> {
        UnsafeEntityCell::new(
            self.world.as_unsafe_world_cell(),
            self.entity,
            self.location,
        )
    }
    fn into_unsafe_entity_cell(self) -> UnsafeEntityCell<'w> {
        UnsafeEntityCell::new(
            self.world.as_unsafe_world_cell(),
            self.entity,
            self.location,
        )
    }

    /// # Safety
    ///
    ///  - `entity` must be valid for `world`: the generation should match that of the entity at the same index.
    ///  - `location` must be sourced from `world`'s `Entities` and must exactly match the location for `entity`
    ///
    ///  The above is trivially satisfied if `location` was sourced from `world.entities().get(entity)`.
    #[inline]
    pub(crate) unsafe fn new(
        world: &'w mut World,
        entity: Entity,
        location: EntityLocation,
    ) -> Self {
        debug_assert!(world.entities().contains(entity));
        debug_assert_eq!(world.entities().get(entity), Some(location));

        EntityWorldMut {
            world,
            entity,
            location,
        }
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Gets metadata indicating the location where the current entity is stored.
    #[inline]
    pub fn location(&self) -> EntityLocation {
        self.location
    }

    /// Returns the archetype that the current entity belongs to.
    #[inline]
    pub fn archetype(&self) -> &Archetype {
        &self.world.archetypes[self.location.archetype_id]
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
    ///     [`Self::contains_type_id`].
    #[inline]
    pub fn contains_id(&self, component_id: ComponentId) -> bool {
        self.as_unsafe_entity_cell_readonly()
            .contains_id(component_id)
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
        self.as_unsafe_entity_cell_readonly()
            .contains_type_id(type_id)
    }

    /// Gets access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'_ T> {
        EntityRef::from(self).get()
    }

    /// Returns read-only components for the current entity that match the query `Q`.
    ///
    /// # Panics
    ///
    /// If the entity does not have the components required by the query `Q`.
    #[inline]
    pub fn components<Q: ReadOnlyQueryData>(&self) -> Q::Item<'_> {
        EntityRef::from(self).components::<Q>()
    }

    /// Returns read-only components for the current entity that match the query `Q`,
    /// or `None` if the entity does not have the components required by the query `Q`.
    #[inline]
    pub fn get_components<Q: ReadOnlyQueryData>(&self) -> Option<Q::Item<'_>> {
        EntityRef::from(self).get_components::<Q>()
    }

    /// Consumes `self` and gets access to the component of type `T` with
    /// the world `'w` lifetime for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn into_borrow<T: Component>(self) -> Option<&'w T> {
        // SAFETY: consuming `self` implies exclusive access
        unsafe { self.into_unsafe_entity_cell().get() }
    }

    /// Gets access to the component of type `T` for the current entity,
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_ref<T: Component>(&self) -> Option<Ref<'_, T>> {
        EntityRef::from(self).get_ref()
    }

    /// Consumes `self` and gets access to the component of type `T`
    /// with the world `'w` lifetime for the current entity,
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn into_ref<T: Component>(self) -> Option<Ref<'w, T>> {
        EntityRef::from(self).get_ref()
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_mut<T: Component>(&mut self) -> Option<Mut<'_, T>> {
        // SAFETY: &mut self implies exclusive access for duration of returned value
        unsafe { self.as_unsafe_entity_cell().get_mut() }
    }

    /// Consumes `self` and gets mutable access to the component of type `T`
    /// with the world `'w` lifetime for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn into_mut<T: Component>(self) -> Option<Mut<'w, T>> {
        // SAFETY: consuming `self` implies exclusive access
        unsafe { self.into_unsafe_entity_cell().get_mut() }
    }

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    #[inline]
    pub fn get_change_ticks<T: Component>(&self) -> Option<ComponentTicks> {
        EntityRef::from(self).get_change_ticks::<T>()
    }

    /// Retrieves the change ticks for the given [`ComponentId`]. This can be useful for implementing change
    /// detection in custom runtimes.
    ///
    /// **You should prefer to use the typed API [`EntityWorldMut::get_change_ticks`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    #[inline]
    pub fn get_change_ticks_by_id(&self, component_id: ComponentId) -> Option<ComponentTicks> {
        EntityRef::from(self).get_change_ticks_by_id(component_id)
    }

    /// Returns [untyped read-only reference(s)](Ptr) to component(s) for the
    /// current entity, based on the given [`ComponentId`]s.
    ///
    /// **You should prefer to use the typed API [`EntityWorldMut::get`] where
    /// possible and only use this in cases where the actual component types
    /// are not known at compile time.**
    ///
    /// Unlike [`EntityWorldMut::get`], this returns untyped reference(s) to
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
        EntityRef::from(self).get_by_id(component_ids)
    }

    /// Consumes `self` and returns [untyped read-only reference(s)](Ptr) to
    /// component(s) with lifetime `'w` for the current entity, based on the
    /// given [`ComponentId`]s.
    ///
    /// **You should prefer to use the typed API [`EntityWorldMut::into_borrow`]
    /// where possible and only use this in cases where the actual component
    /// types are not known at compile time.**
    ///
    /// Unlike [`EntityWorldMut::into_borrow`], this returns untyped reference(s) to
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
        // SAFETY:
        // - We have read-only access to all components of this entity.
        // - consuming `self` ensures that no references exist to this entity's components.
        unsafe { component_ids.fetch_ref(self.into_unsafe_entity_cell()) }
    }

    /// Returns [untyped mutable reference(s)](MutUntyped) to component(s) for
    /// the current entity, based on the given [`ComponentId`]s.
    ///
    /// **You should prefer to use the typed API [`EntityWorldMut::get_mut`] where
    /// possible and only use this in cases where the actual component types
    /// are not known at compile time.**
    ///
    /// Unlike [`EntityWorldMut::get_mut`], this returns untyped reference(s) to
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
    pub fn get_mut_by_id<F: DynamicComponentFetch>(
        &mut self,
        component_ids: F,
    ) -> Result<F::Mut<'_>, EntityComponentError> {
        // SAFETY:
        // - `&mut self` ensures that no references exist to this entity's components.
        // - We have exclusive access to all components of this entity.
        unsafe { component_ids.fetch_mut(self.as_unsafe_entity_cell()) }
    }

    /// Consumes `self` and returns [untyped mutable reference(s)](MutUntyped)
    /// to component(s) with lifetime `'w` for the current entity, based on the
    /// given [`ComponentId`]s.
    ///
    /// **You should prefer to use the typed API [`EntityWorldMut::into_mut`] where
    /// possible and only use this in cases where the actual component types
    /// are not known at compile time.**
    ///
    /// Unlike [`EntityWorldMut::into_mut`], this returns untyped reference(s) to
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
        // - consuming `self` ensures that no references exist to this entity's components.
        // - We have exclusive access to all components of this entity.
        unsafe { component_ids.fetch_mut(self.into_unsafe_entity_cell()) }
    }

    /// Adds a [`Bundle`] of components to the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    #[track_caller]
    pub fn insert<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        self.insert_with_caller(
            bundle,
            InsertMode::Replace,
            #[cfg(feature = "track_change_detection")]
            core::panic::Location::caller(),
        )
    }

    /// Adds a [`Bundle`] of components to the entity without overwriting.
    ///
    /// This will leave any previous value(s) of the same component type
    /// unchanged.
    #[track_caller]
    pub fn insert_if_new<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        self.insert_with_caller(
            bundle,
            InsertMode::Keep,
            #[cfg(feature = "track_change_detection")]
            core::panic::Location::caller(),
        )
    }

    /// Split into a new function so we can pass the calling location into the function when using
    /// as a command.
    #[inline]
    pub(crate) fn insert_with_caller<T: Bundle>(
        &mut self,
        bundle: T,
        mode: InsertMode,
        #[cfg(feature = "track_change_detection")] caller: &'static core::panic::Location,
    ) -> &mut Self {
        let change_tick = self.world.change_tick();
        let mut bundle_inserter =
            BundleInserter::new::<T>(self.world, self.location.archetype_id, change_tick);
        self.location =
            // SAFETY: location matches current entity. `T` matches `bundle_info`
            unsafe {
                bundle_inserter.insert(self.entity, self.location, bundle, mode, #[cfg(feature = "track_change_detection")] caller)
            };
        self
    }

    /// Inserts a dynamic [`Component`] into the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    ///
    /// You should prefer to use the typed API [`EntityWorldMut::insert`] where possible.
    ///
    /// # Safety
    ///
    /// - [`ComponentId`] must be from the same world as [`EntityWorldMut`]
    /// - [`OwningPtr`] must be a valid reference to the type represented by [`ComponentId`]
    #[track_caller]
    pub unsafe fn insert_by_id(
        &mut self,
        component_id: ComponentId,
        component: OwningPtr<'_>,
    ) -> &mut Self {
        let change_tick = self.world.change_tick();
        let bundle_id = self
            .world
            .bundles
            .init_component_info(&self.world.components, component_id);
        let storage_type = self.world.bundles.get_storage_unchecked(bundle_id);

        let bundle_inserter = BundleInserter::new_with_id(
            self.world,
            self.location.archetype_id,
            bundle_id,
            change_tick,
        );

        self.location = insert_dynamic_bundle(
            bundle_inserter,
            self.entity,
            self.location,
            Some(component).into_iter(),
            Some(storage_type).iter().cloned(),
        );
        self
    }

    /// Inserts a dynamic [`Bundle`] into the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    ///
    /// You should prefer to use the typed API [`EntityWorldMut::insert`] where possible.
    /// If your [`Bundle`] only has one component, use the cached API [`EntityWorldMut::insert_by_id`].
    ///
    /// If possible, pass a sorted slice of `ComponentId` to maximize caching potential.
    ///
    /// # Safety
    /// - Each [`ComponentId`] must be from the same world as [`EntityWorldMut`]
    /// - Each [`OwningPtr`] must be a valid reference to the type represented by [`ComponentId`]
    #[track_caller]
    pub unsafe fn insert_by_ids<'a, I: Iterator<Item = OwningPtr<'a>>>(
        &mut self,
        component_ids: &[ComponentId],
        iter_components: I,
    ) -> &mut Self {
        let change_tick = self.world.change_tick();
        let bundle_id = self
            .world
            .bundles
            .init_dynamic_info(&self.world.components, component_ids);
        let mut storage_types =
            core::mem::take(self.world.bundles.get_storages_unchecked(bundle_id));
        let bundle_inserter = BundleInserter::new_with_id(
            self.world,
            self.location.archetype_id,
            bundle_id,
            change_tick,
        );

        self.location = insert_dynamic_bundle(
            bundle_inserter,
            self.entity,
            self.location,
            iter_components,
            (*storage_types).iter().cloned(),
        );
        *self.world.bundles.get_storages_unchecked(bundle_id) = core::mem::take(&mut storage_types);
        self
    }

    /// Removes all components in the [`Bundle`] from the entity and returns their previous values.
    ///
    /// **Note:** If the entity does not have every component in the bundle, this method will not
    /// remove any of them.
    // TODO: BundleRemover?
    #[must_use]
    pub fn take<T: Bundle>(&mut self) -> Option<T> {
        let world = &mut self.world;
        let storages = &mut world.storages;
        let components = &mut world.components;
        let bundle_id = world.bundles.register_info::<T>(components, storages);
        // SAFETY: We just ensured this bundle exists
        let bundle_info = unsafe { world.bundles.get_unchecked(bundle_id) };
        let old_location = self.location;
        // SAFETY: `archetype_id` exists because it is referenced in the old `EntityLocation` which is valid,
        // components exist in `bundle_info` because `Bundles::init_info` initializes a `BundleInfo` containing all components of the bundle type `T`
        let new_archetype_id = unsafe {
            remove_bundle_from_archetype(
                &mut world.archetypes,
                storages,
                components,
                &world.observers,
                old_location.archetype_id,
                bundle_info,
                false,
            )?
        };

        if new_archetype_id == old_location.archetype_id {
            return None;
        }

        let entity = self.entity;
        // SAFETY: Archetypes and Bundles cannot be mutably aliased through DeferredWorld
        let (old_archetype, bundle_info, mut deferred_world) = unsafe {
            let bundle_info: *const BundleInfo = bundle_info;
            let world = world.as_unsafe_world_cell();
            (
                &world.archetypes()[old_location.archetype_id],
                &*bundle_info,
                world.into_deferred(),
            )
        };

        // SAFETY: all bundle components exist in World
        unsafe {
            trigger_on_replace_and_on_remove_hooks_and_observers(
                &mut deferred_world,
                old_archetype,
                entity,
                bundle_info,
            );
        }

        let archetypes = &mut world.archetypes;
        let storages = &mut world.storages;
        let components = &mut world.components;
        let entities = &mut world.entities;
        let removed_components = &mut world.removed_components;

        let entity = self.entity;
        let mut bundle_components = bundle_info.iter_explicit_components();
        // SAFETY: bundle components are iterated in order, which guarantees that the component type
        // matches
        let result = unsafe {
            T::from_components(storages, &mut |storages| {
                let component_id = bundle_components.next().unwrap();
                // SAFETY:
                // - entity location is valid
                // - table row is removed below, without dropping the contents
                // - `components` comes from the same world as `storages`
                take_component(
                    storages,
                    components,
                    removed_components,
                    component_id,
                    entity,
                    old_location,
                )
            })
        };

        #[allow(clippy::undocumented_unsafe_blocks)] // TODO: document why this is safe
        unsafe {
            Self::move_entity_from_remove::<false>(
                entity,
                &mut self.location,
                old_location.archetype_id,
                old_location,
                entities,
                archetypes,
                storages,
                new_archetype_id,
            );
        }
        Some(result)
    }

    /// # Safety
    ///
    /// `new_archetype_id` must have the same or a subset of the components
    /// in `old_archetype_id`. Probably more safety stuff too, audit a call to
    /// this fn as if the code here was written inline
    ///
    /// when DROP is true removed components will be dropped otherwise they will be forgotten
    // We use a const generic here so that we are less reliant on
    // inlining for rustc to optimize out the `match DROP`
    #[allow(clippy::too_many_arguments)]
    unsafe fn move_entity_from_remove<const DROP: bool>(
        entity: Entity,
        self_location: &mut EntityLocation,
        old_archetype_id: ArchetypeId,
        old_location: EntityLocation,
        entities: &mut Entities,
        archetypes: &mut Archetypes,
        storages: &mut Storages,
        new_archetype_id: ArchetypeId,
    ) {
        let old_archetype = &mut archetypes[old_archetype_id];
        let remove_result = old_archetype.swap_remove(old_location.archetype_row);
        // if an entity was moved into this entity's archetype row, update its archetype row
        if let Some(swapped_entity) = remove_result.swapped_entity {
            let swapped_location = entities.get(swapped_entity).unwrap();

            entities.set(
                swapped_entity.index(),
                EntityLocation {
                    archetype_id: swapped_location.archetype_id,
                    archetype_row: old_location.archetype_row,
                    table_id: swapped_location.table_id,
                    table_row: swapped_location.table_row,
                },
            );
        }
        let old_table_row = remove_result.table_row;
        let old_table_id = old_archetype.table_id();
        let new_archetype = &mut archetypes[new_archetype_id];

        let new_location = if old_table_id == new_archetype.table_id() {
            new_archetype.allocate(entity, old_table_row)
        } else {
            let (old_table, new_table) = storages
                .tables
                .get_2_mut(old_table_id, new_archetype.table_id());

            let move_result = if DROP {
                // SAFETY: old_table_row exists
                unsafe { old_table.move_to_and_drop_missing_unchecked(old_table_row, new_table) }
            } else {
                // SAFETY: old_table_row exists
                unsafe { old_table.move_to_and_forget_missing_unchecked(old_table_row, new_table) }
            };

            // SAFETY: move_result.new_row is a valid position in new_archetype's table
            let new_location = unsafe { new_archetype.allocate(entity, move_result.new_row) };

            // if an entity was moved into this entity's table row, update its table row
            if let Some(swapped_entity) = move_result.swapped_entity {
                let swapped_location = entities.get(swapped_entity).unwrap();

                entities.set(
                    swapped_entity.index(),
                    EntityLocation {
                        archetype_id: swapped_location.archetype_id,
                        archetype_row: swapped_location.archetype_row,
                        table_id: swapped_location.table_id,
                        table_row: old_location.table_row,
                    },
                );
                archetypes[swapped_location.archetype_id]
                    .set_entity_table_row(swapped_location.archetype_row, old_table_row);
            }

            new_location
        };

        *self_location = new_location;
        // SAFETY: The entity is valid and has been moved to the new location already.
        unsafe {
            entities.set(entity.index(), new_location);
        }
    }

    /// Remove the components of `bundle` from `entity`.
    ///
    /// # Safety
    /// - A `BundleInfo` with the corresponding `BundleId` must have been initialized.
    #[allow(clippy::too_many_arguments)]
    unsafe fn remove_bundle(&mut self, bundle: BundleId) -> EntityLocation {
        let entity = self.entity;
        let world = &mut self.world;
        let location = self.location;
        // SAFETY: the caller guarantees that the BundleInfo for this id has been initialized.
        let bundle_info = world.bundles.get_unchecked(bundle);

        // SAFETY: `archetype_id` exists because it is referenced in `location` which is valid
        // and components in `bundle_info` must exist due to this function's safety invariants.
        let new_archetype_id = remove_bundle_from_archetype(
            &mut world.archetypes,
            &mut world.storages,
            &world.components,
            &world.observers,
            location.archetype_id,
            bundle_info,
            // components from the bundle that are not present on the entity are ignored
            true,
        )
        .expect("intersections should always return a result");

        if new_archetype_id == location.archetype_id {
            return location;
        }

        // SAFETY: Archetypes and Bundles cannot be mutably aliased through DeferredWorld
        let (old_archetype, bundle_info, mut deferred_world) = unsafe {
            let bundle_info: *const BundleInfo = bundle_info;
            let world = world.as_unsafe_world_cell();
            (
                &world.archetypes()[location.archetype_id],
                &*bundle_info,
                world.into_deferred(),
            )
        };

        // SAFETY: all bundle components exist in World
        unsafe {
            trigger_on_replace_and_on_remove_hooks_and_observers(
                &mut deferred_world,
                old_archetype,
                entity,
                bundle_info,
            );
        }

        let old_archetype = &world.archetypes[location.archetype_id];
        for component_id in bundle_info.iter_explicit_components() {
            if old_archetype.contains(component_id) {
                world.removed_components.send(component_id, entity);

                // Make sure to drop components stored in sparse sets.
                // Dense components are dropped later in `move_to_and_drop_missing_unchecked`.
                if let Some(StorageType::SparseSet) = old_archetype.get_storage_type(component_id) {
                    world
                        .storages
                        .sparse_sets
                        .get_mut(component_id)
                        .unwrap()
                        .remove(entity);
                }
            }
        }

        // SAFETY: `new_archetype_id` is a subset of the components in `old_location.archetype_id`
        // because it is created by removing a bundle from these components.
        let mut new_location = location;
        Self::move_entity_from_remove::<true>(
            entity,
            &mut new_location,
            location.archetype_id,
            location,
            &mut world.entities,
            &mut world.archetypes,
            &mut world.storages,
            new_archetype_id,
        );

        new_location
    }

    /// Removes any components in the [`Bundle`] from the entity.
    ///
    /// See [`EntityCommands::remove`](crate::system::EntityCommands::remove) for more details.
    // TODO: BundleRemover?
    pub fn remove<T: Bundle>(&mut self) -> &mut Self {
        let storages = &mut self.world.storages;
        let components = &mut self.world.components;
        let bundle_info = self.world.bundles.register_info::<T>(components, storages);

        // SAFETY: the `BundleInfo` is initialized above
        self.location = unsafe { self.remove_bundle(bundle_info) };

        self
    }

    /// Removes all components in the [`Bundle`] and remove all required components for each component in the bundle
    pub fn remove_with_requires<T: Bundle>(&mut self) -> &mut Self {
        let storages = &mut self.world.storages;
        let components = &mut self.world.components;
        let bundles = &mut self.world.bundles;

        let bundle_id = bundles.register_contributed_bundle_info::<T>(components, storages);

        // SAFETY: the dynamic `BundleInfo` is initialized above
        self.location = unsafe { self.remove_bundle(bundle_id) };

        self
    }

    /// Removes any components except those in the [`Bundle`] (and its Required Components) from the entity.
    ///
    /// See [`EntityCommands::retain`](crate::system::EntityCommands::retain) for more details.
    pub fn retain<T: Bundle>(&mut self) -> &mut Self {
        let archetypes = &mut self.world.archetypes;
        let storages = &mut self.world.storages;
        let components = &mut self.world.components;

        let retained_bundle = self.world.bundles.register_info::<T>(components, storages);
        // SAFETY: `retained_bundle` exists as we just initialized it.
        let retained_bundle_info = unsafe { self.world.bundles.get_unchecked(retained_bundle) };
        let old_location = self.location;
        let old_archetype = &mut archetypes[old_location.archetype_id];

        // PERF: this could be stored in an Archetype Edge
        let to_remove = &old_archetype
            .components()
            .filter(|c| !retained_bundle_info.contributed_components().contains(c))
            .collect::<Vec<_>>();
        let remove_bundle = self.world.bundles.init_dynamic_info(components, to_remove);

        // SAFETY: the `BundleInfo` for the components to remove is initialized above
        self.location = unsafe { self.remove_bundle(remove_bundle) };
        self
    }

    /// Removes a dynamic [`Component`] from the entity if it exists.
    ///
    /// You should prefer to use the typed API [`EntityWorldMut::remove`] where possible.
    ///
    /// # Panics
    ///
    /// Panics if the provided [`ComponentId`] does not exist in the [`World`].
    pub fn remove_by_id(&mut self, component_id: ComponentId) -> &mut Self {
        let components = &mut self.world.components;

        let bundle_id = self
            .world
            .bundles
            .init_component_info(components, component_id);

        // SAFETY: the `BundleInfo` for this `component_id` is initialized above
        self.location = unsafe { self.remove_bundle(bundle_id) };

        self
    }

    /// Removes all components associated with the entity.
    pub fn clear(&mut self) -> &mut Self {
        let component_ids: Vec<ComponentId> = self.archetype().components().collect();
        let components = &mut self.world.components;

        let bundle_id = self
            .world
            .bundles
            .init_dynamic_info(components, component_ids.as_slice());

        // SAFETY: the `BundleInfo` for this `component_id` is initialized above
        self.location = unsafe { self.remove_bundle(bundle_id) };

        self
    }

    /// Despawns the current entity.
    ///
    /// See [`World::despawn`] for more details.
    pub fn despawn(self) {
        let world = self.world;
        let archetype = &world.archetypes[self.location.archetype_id];

        // SAFETY: Archetype cannot be mutably aliased by DeferredWorld
        let (archetype, mut deferred_world) = unsafe {
            let archetype: *const Archetype = archetype;
            let world = world.as_unsafe_world_cell();
            (&*archetype, world.into_deferred())
        };

        // SAFETY: All components in the archetype exist in world
        unsafe {
            deferred_world.trigger_on_replace(archetype, self.entity, archetype.components());
            if archetype.has_replace_observer() {
                deferred_world.trigger_observers(ON_REPLACE, self.entity, archetype.components());
            }
            deferred_world.trigger_on_remove(archetype, self.entity, archetype.components());
            if archetype.has_remove_observer() {
                deferred_world.trigger_observers(ON_REMOVE, self.entity, archetype.components());
            }
        }

        for component_id in archetype.components() {
            world.removed_components.send(component_id, self.entity);
        }

        // Observers and on_remove hooks may reserve new entities, which
        // requires a flush before Entities::free may be called.
        world.flush_entities();

        let location = world
            .entities
            .free(self.entity)
            .expect("entity should exist at this point.");
        let table_row;
        let moved_entity;

        {
            let archetype = &mut world.archetypes[self.location.archetype_id];
            let remove_result = archetype.swap_remove(location.archetype_row);
            if let Some(swapped_entity) = remove_result.swapped_entity {
                let swapped_location = world.entities.get(swapped_entity).unwrap();
                // SAFETY: swapped_entity is valid and the swapped entity's components are
                // moved to the new location immediately after.
                unsafe {
                    world.entities.set(
                        swapped_entity.index(),
                        EntityLocation {
                            archetype_id: swapped_location.archetype_id,
                            archetype_row: location.archetype_row,
                            table_id: swapped_location.table_id,
                            table_row: swapped_location.table_row,
                        },
                    );
                }
            }
            table_row = remove_result.table_row;

            for component_id in archetype.sparse_set_components() {
                let sparse_set = world.storages.sparse_sets.get_mut(component_id).unwrap();
                sparse_set.remove(self.entity);
            }
            // SAFETY: table rows stored in archetypes always exist
            moved_entity = unsafe {
                world.storages.tables[archetype.table_id()].swap_remove_unchecked(table_row)
            };
        };

        if let Some(moved_entity) = moved_entity {
            let moved_location = world.entities.get(moved_entity).unwrap();
            // SAFETY: `moved_entity` is valid and the provided `EntityLocation` accurately reflects
            //         the current location of the entity and its component data.
            unsafe {
                world.entities.set(
                    moved_entity.index(),
                    EntityLocation {
                        archetype_id: moved_location.archetype_id,
                        archetype_row: moved_location.archetype_row,
                        table_id: moved_location.table_id,
                        table_row,
                    },
                );
            }
            world.archetypes[moved_location.archetype_id]
                .set_entity_table_row(moved_location.archetype_row, table_row);
        }
        world.flush();
    }

    /// Ensures any commands triggered by the actions of Self are applied, equivalent to [`World::flush`]
    pub fn flush(self) -> Entity {
        self.world.flush();
        self.entity
    }

    /// Gets read-only access to the world that the current entity belongs to.
    #[inline]
    pub fn world(&self) -> &World {
        self.world
    }

    /// Returns this entity's world.
    ///
    /// See [`EntityWorldMut::world_scope`] or [`EntityWorldMut::into_world_mut`] for a safe alternative.
    ///
    /// # Safety
    /// Caller must not modify the world in a way that changes the current entity's location
    /// If the caller _does_ do something that could change the location, `self.update_location()`
    /// must be called before using any other methods on this [`EntityWorldMut`].
    #[inline]
    pub unsafe fn world_mut(&mut self) -> &mut World {
        self.world
    }

    /// Returns this entity's [`World`], consuming itself.
    #[inline]
    pub fn into_world_mut(self) -> &'w mut World {
        self.world
    }

    /// Gives mutable access to this entity's [`World`] in a temporary scope.
    /// This is a safe alternative to using [`EntityWorldMut::world_mut`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Resource, Default, Clone, Copy)]
    /// struct R(u32);
    ///
    /// # let mut world = World::new();
    /// # world.init_resource::<R>();
    /// # let mut entity = world.spawn_empty();
    /// // This closure gives us temporary access to the world.
    /// let new_r = entity.world_scope(|world: &mut World| {
    ///     // Mutate the world while we have access to it.
    ///     let mut r = world.resource_mut::<R>();
    ///     r.0 += 1;
    ///
    ///     // Return a value from the world before giving it back to the `EntityWorldMut`.
    ///     *r
    /// });
    /// # assert_eq!(new_r.0, 1);
    /// ```
    pub fn world_scope<U>(&mut self, f: impl FnOnce(&mut World) -> U) -> U {
        struct Guard<'w, 'a> {
            entity_mut: &'a mut EntityWorldMut<'w>,
        }

        impl Drop for Guard<'_, '_> {
            #[inline]
            fn drop(&mut self) {
                self.entity_mut.update_location();
            }
        }

        // When `guard` is dropped at the end of this scope,
        // it will update the cached `EntityLocation` for this instance.
        // This will run even in case the closure `f` unwinds.
        let guard = Guard { entity_mut: self };
        f(guard.entity_mut.world)
    }

    /// Updates the internal entity location to match the current location in the internal
    /// [`World`].
    ///
    /// This is *only* required when using the unsafe function [`EntityWorldMut::world_mut`],
    /// which enables the location to change.
    pub fn update_location(&mut self) {
        self.location = self.world.entities().get(self.entity).unwrap();
    }

    /// Gets an Entry into the world for this entity and component for in-place manipulation.
    ///
    /// The type parameter specifies which component to get.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    /// entity.entry().or_insert_with(|| Comp(4));
    /// # let entity_id = entity.id();
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 4);
    ///
    /// # let mut entity = world.get_entity_mut(entity_id).unwrap();
    /// entity.entry::<Comp>().and_modify(|mut c| c.0 += 1);
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 5);
    /// ```
    pub fn entry<'a, T: Component>(&'a mut self) -> Entry<'w, 'a, T> {
        if self.contains::<T>() {
            Entry::Occupied(OccupiedEntry {
                entity_world: self,
                _marker: PhantomData,
            })
        } else {
            Entry::Vacant(VacantEntry {
                entity_world: self,
                _marker: PhantomData,
            })
        }
    }

    /// Triggers the given `event` for this entity, which will run any observers watching for it.
    pub fn trigger(&mut self, event: impl Event) -> &mut Self {
        self.world.trigger_targets(event, self.entity);
        self
    }

    /// Creates an [`Observer`] listening for events of type `E` targeting this entity.
    /// In order to trigger the callback the entity must also match the query when the event is fired.
    pub fn observe_entity<E: Event, B: Bundle, M>(
        &mut self,
        observer: impl IntoObserverSystem<E, B, M>,
    ) -> &mut Self {
        self.world
            .spawn(Observer::new(observer).with_entity(self.entity));
        self
    }
}

/// # Safety
/// All components in the archetype must exist in world
unsafe fn trigger_on_replace_and_on_remove_hooks_and_observers(
    deferred_world: &mut DeferredWorld,
    archetype: &Archetype,
    entity: Entity,
    bundle_info: &BundleInfo,
) {
    deferred_world.trigger_on_replace(archetype, entity, bundle_info.iter_explicit_components());
    if archetype.has_replace_observer() {
        deferred_world.trigger_observers(
            ON_REPLACE,
            entity,
            bundle_info.iter_explicit_components(),
        );
    }
    deferred_world.trigger_on_remove(archetype, entity, bundle_info.iter_explicit_components());
    if archetype.has_remove_observer() {
        deferred_world.trigger_observers(ON_REMOVE, entity, bundle_info.iter_explicit_components());
    }
}

const QUERY_MISMATCH_ERROR: &str = "Query does not match the current entity";

/// A view into a single entity and component in a world, which may either be vacant or occupied.
///
/// This `enum` can only be constructed from the [`entry`] method on [`EntityWorldMut`].
///
/// [`entry`]: EntityWorldMut::entry
pub enum Entry<'w, 'a, T: Component> {
    /// An occupied entry.
    Occupied(OccupiedEntry<'w, 'a, T>),
    /// A vacant entry.
    Vacant(VacantEntry<'w, 'a, T>),
}

impl<'w, 'a, T: Component> Entry<'w, 'a, T> {
    /// Provides in-place mutable access to an occupied entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(0));
    ///
    /// entity.entry::<Comp>().and_modify(|mut c| c.0 += 1);
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 1);
    /// ```
    #[inline]
    pub fn and_modify<F: FnOnce(Mut<'_, T>)>(self, f: F) -> Self {
        match self {
            Entry::Occupied(mut entry) => {
                f(entry.get_mut());
                Entry::Occupied(entry)
            }
            Entry::Vacant(entry) => Entry::Vacant(entry),
        }
    }

    /// Replaces the component of the entry, and returns an [`OccupiedEntry`].
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// let entry = entity.entry().insert_entry(Comp(4));
    /// assert_eq!(entry.get(), &Comp(4));
    ///
    /// let entry = entity.entry().insert_entry(Comp(2));
    /// assert_eq!(entry.get(), &Comp(2));
    /// ```
    #[inline]
    pub fn insert_entry(self, component: T) -> OccupiedEntry<'w, 'a, T> {
        match self {
            Entry::Occupied(mut entry) => {
                entry.insert(component);
                entry
            }
            Entry::Vacant(entry) => entry.insert_entry(component),
        }
    }

    /// Ensures the entry has this component by inserting the given default if empty, and
    /// returns a mutable reference to this component in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// entity.entry().or_insert(Comp(4));
    /// # let entity_id = entity.id();
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 4);
    ///
    /// # let mut entity = world.get_entity_mut(entity_id).unwrap();
    /// entity.entry().or_insert(Comp(15)).0 *= 2;
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 8);
    /// ```
    #[inline]
    pub fn or_insert(self, default: T) -> Mut<'a, T> {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default),
        }
    }

    /// Ensures the entry has this component by inserting the result of the default function if
    /// empty, and returns a mutable reference to this component in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// entity.entry().or_insert_with(|| Comp(4));
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 4);
    /// ```
    #[inline]
    pub fn or_insert_with<F: FnOnce() -> T>(self, default: F) -> Mut<'a, T> {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(default()),
        }
    }
}

impl<'w, 'a, T: Component + Default> Entry<'w, 'a, T> {
    /// Ensures the entry has this component by inserting the default value if empty, and
    /// returns a mutable reference to this component in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// entity.entry::<Comp>().or_default();
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 0);
    /// ```
    #[inline]
    pub fn or_default(self) -> Mut<'a, T> {
        match self {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => entry.insert(Default::default()),
        }
    }
}

/// A view into an occupied entry in a [`EntityWorldMut`]. It is part of the [`Entry`] enum.
///
/// The contained entity must have the component type parameter if we have this struct.
pub struct OccupiedEntry<'w, 'a, T: Component> {
    entity_world: &'a mut EntityWorldMut<'w>,
    _marker: PhantomData<T>,
}

impl<'w, 'a, T: Component> OccupiedEntry<'w, 'a, T> {
    /// Gets a reference to the component in the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::Entry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(5));
    ///
    /// if let Entry::Occupied(o) = entity.entry::<Comp>() {
    ///     assert_eq!(o.get().0, 5);
    /// }
    /// ```
    #[inline]
    pub fn get(&self) -> &T {
        // This shouldn't panic because if we have an OccupiedEntry the component must exist.
        self.entity_world.get::<T>().unwrap()
    }

    /// Gets a mutable reference to the component in the entry.
    ///
    /// If you need a reference to the `OccupiedEntry` which may outlive the destruction of
    /// the `Entry` value, see [`into_mut`].
    ///
    /// [`into_mut`]: Self::into_mut
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::Entry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(5));
    ///
    /// if let Entry::Occupied(mut o) = entity.entry::<Comp>() {
    ///     o.get_mut().0 += 10;
    ///     assert_eq!(o.get().0, 15);
    ///
    ///     // We can use the same Entry multiple times.
    ///     o.get_mut().0 += 2
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 17);
    /// ```
    #[inline]
    pub fn get_mut(&mut self) -> Mut<'_, T> {
        // This shouldn't panic because if we have an OccupiedEntry the component must exist.
        self.entity_world.get_mut::<T>().unwrap()
    }

    /// Converts the `OccupiedEntry` into a mutable reference to the value in the entry with
    /// a lifetime bound to the `EntityWorldMut`.
    ///
    /// If you need multiple references to the `OccupiedEntry`, see [`get_mut`].
    ///
    /// [`get_mut`]: Self::get_mut
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::Entry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(5));
    ///
    /// if let Entry::Occupied(o) = entity.entry::<Comp>() {
    ///     o.into_mut().0 += 10;
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 15);
    /// ```
    #[inline]
    pub fn into_mut(self) -> Mut<'a, T> {
        // This shouldn't panic because if we have an OccupiedEntry the component must exist.
        self.entity_world.get_mut().unwrap()
    }

    /// Replaces the component of the entry.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::Entry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(5));
    ///
    /// if let Entry::Occupied(mut o) = entity.entry::<Comp>() {
    ///     o.insert(Comp(10));
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 10);
    /// ```
    #[inline]
    pub fn insert(&mut self, component: T) {
        self.entity_world.insert(component);
    }

    /// Removes the component from the entry and returns it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::Entry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn(Comp(5));
    ///
    /// if let Entry::Occupied(o) = entity.entry::<Comp>() {
    ///     assert_eq!(o.take(), Comp(5));
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().iter(&world).len(), 0);
    /// ```
    #[inline]
    pub fn take(self) -> T {
        // This shouldn't panic because if we have an OccupiedEntry the component must exist.
        self.entity_world.take().unwrap()
    }
}

/// A view into a vacant entry in a [`EntityWorldMut`]. It is part of the [`Entry`] enum.
pub struct VacantEntry<'w, 'a, T: Component> {
    entity_world: &'a mut EntityWorldMut<'w>,
    _marker: PhantomData<T>,
}

impl<'w, 'a, T: Component> VacantEntry<'w, 'a, T> {
    /// Inserts the component into the `VacantEntry` and returns a mutable reference to it.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::Entry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// if let Entry::Vacant(v) = entity.entry::<Comp>() {
    ///     v.insert(Comp(10));
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 10);
    /// ```
    #[inline]
    pub fn insert(self, component: T) -> Mut<'a, T> {
        self.entity_world.insert(component);
        // This shouldn't panic because we just added this component
        self.entity_world.get_mut::<T>().unwrap()
    }

    /// Inserts the component into the `VacantEntry` and returns an `OccupiedEntry`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use bevy_ecs::{prelude::*, world::Entry};
    /// #[derive(Component, Default, Clone, Copy, Debug, PartialEq)]
    /// struct Comp(u32);
    ///
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    ///
    /// if let Entry::Vacant(v) = entity.entry::<Comp>() {
    ///     v.insert_entry(Comp(10));
    /// }
    ///
    /// assert_eq!(world.query::<&Comp>().single(&world).0, 10);
    /// ```
    #[inline]
    pub fn insert_entry(self, component: T) -> OccupiedEntry<'w, 'a, T> {
        self.entity_world.insert(component);
        OccupiedEntry {
            entity_world: self.entity_world,
            _marker: PhantomData,
        }
    }
}

/// Provides read-only access to a single entity and some of its components defined by the contained [`Access`].
///
/// To define the access when used as a [`QueryData`](crate::query::QueryData),
/// use a [`QueryBuilder`](crate::query::QueryBuilder) or [`QueryParamBuilder`](crate::system::QueryParamBuilder).
/// The `FilteredEntityRef` must be the entire `QueryData`, and not nested inside a tuple with other data.
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
/// let filtered_entity: FilteredEntityRef = query.single(&mut world);
/// let component: &A = filtered_entity.get().unwrap();
///
/// // Here `FilteredEntityRef` is nested in a tuple, so it does not have access to `&A`.
/// let mut query = QueryBuilder::<(Entity, FilteredEntityRef)>::new(&mut world)
///     .data::<&A>()
///     .build();
///
/// let (_, filtered_entity) = query.single(&mut world);
/// assert!(filtered_entity.get::<A>().is_none());
/// ```
#[derive(Clone)]
pub struct FilteredEntityRef<'w> {
    entity: UnsafeEntityCell<'w>,
    access: Access<ComponentId>,
}

impl<'w> FilteredEntityRef<'w> {
    /// # Safety
    /// - No `&mut World` can exist from the underlying `UnsafeWorldCell`
    /// - If `access` takes read access to a component no mutable reference to that
    ///     component can exist at the same time as the returned [`FilteredEntityMut`]
    /// - If `access` takes any access for a component `entity` must have that component.
    #[inline]
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>, access: Access<ComponentId>) -> Self {
        Self { entity, access }
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
    pub fn access(&self) -> &Access<ComponentId> {
        &self.access
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
    ///     [`Self::contains_type_id`].
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
        let id = self.entity.world().components().get_id(TypeId::of::<T>())?;
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
        let id = self.entity.world().components().get_id(TypeId::of::<T>())?;
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
        let id = self.entity.world().components().get_id(TypeId::of::<T>())?;
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
}

impl<'w> From<FilteredEntityMut<'w>> for FilteredEntityRef<'w> {
    #[inline]
    fn from(entity_mut: FilteredEntityMut<'w>) -> Self {
        // SAFETY:
        // - `FilteredEntityMut` guarantees exclusive access to all components in the new `FilteredEntityRef`.
        unsafe { FilteredEntityRef::new(entity_mut.entity, entity_mut.access) }
    }
}

impl<'a> From<&'a FilteredEntityMut<'_>> for FilteredEntityRef<'a> {
    #[inline]
    fn from(entity_mut: &'a FilteredEntityMut<'_>) -> Self {
        // SAFETY:
        // - `FilteredEntityMut` guarantees exclusive access to all components in the new `FilteredEntityRef`.
        unsafe { FilteredEntityRef::new(entity_mut.entity, entity_mut.access.clone()) }
    }
}

impl<'a> From<EntityRef<'a>> for FilteredEntityRef<'a> {
    fn from(entity: EntityRef<'a>) -> Self {
        // SAFETY:
        // - `EntityRef` guarantees exclusive access to all components in the new `FilteredEntityRef`.
        unsafe {
            let mut access = Access::default();
            access.read_all();
            FilteredEntityRef::new(entity.0, access)
        }
    }
}

impl<'a> From<&'a EntityRef<'_>> for FilteredEntityRef<'a> {
    fn from(entity: &'a EntityRef<'_>) -> Self {
        // SAFETY:
        // - `EntityRef` guarantees exclusive access to all components in the new `FilteredEntityRef`.
        unsafe {
            let mut access = Access::default();
            access.read_all();
            FilteredEntityRef::new(entity.0, access)
        }
    }
}

impl<'a> From<EntityMut<'a>> for FilteredEntityRef<'a> {
    fn from(entity: EntityMut<'a>) -> Self {
        // SAFETY:
        // - `EntityMut` guarantees exclusive access to all components in the new `FilteredEntityRef`.
        unsafe {
            let mut access = Access::default();
            access.read_all();
            FilteredEntityRef::new(entity.0, access)
        }
    }
}

impl<'a> From<&'a EntityMut<'_>> for FilteredEntityRef<'a> {
    fn from(entity: &'a EntityMut<'_>) -> Self {
        // SAFETY:
        // - `EntityMut` guarantees exclusive access to all components in the new `FilteredEntityRef`.
        unsafe {
            let mut access = Access::default();
            access.read_all();
            FilteredEntityRef::new(entity.0, access)
        }
    }
}

impl<'a> From<EntityWorldMut<'a>> for FilteredEntityRef<'a> {
    fn from(entity: EntityWorldMut<'a>) -> Self {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe {
            let mut access = Access::default();
            access.read_all();
            FilteredEntityRef::new(entity.into_unsafe_entity_cell(), access)
        }
    }
}

impl<'a> From<&'a EntityWorldMut<'_>> for FilteredEntityRef<'a> {
    fn from(entity: &'a EntityWorldMut<'_>) -> Self {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe {
            let mut access = Access::default();
            access.read_all();
            FilteredEntityRef::new(entity.as_unsafe_entity_cell_readonly(), access)
        }
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
/// let mut filtered_entity: FilteredEntityMut = query.single_mut(&mut world);
/// let component: Mut<A> = filtered_entity.get_mut().unwrap();
///
/// // Here `FilteredEntityMut` is nested in a tuple, so it does not have access to `&mut A`.
/// let mut query = QueryBuilder::<(Entity, FilteredEntityMut)>::new(&mut world)
///     .data::<&mut A>()
///     .build();
///
/// let (_, mut filtered_entity) = query.single_mut(&mut world);
/// assert!(filtered_entity.get_mut::<A>().is_none());
/// ```
pub struct FilteredEntityMut<'w> {
    entity: UnsafeEntityCell<'w>,
    access: Access<ComponentId>,
}

impl<'w> FilteredEntityMut<'w> {
    /// # Safety
    /// - No `&mut World` can exist from the underlying `UnsafeWorldCell`
    /// - If `access` takes read access to a component no mutable reference to that
    ///     component can exist at the same time as the returned [`FilteredEntityMut`]
    /// - If `access` takes write access to a component, no reference to that component
    ///     may exist at the same time as the returned [`FilteredEntityMut`]
    /// - If `access` takes any access for a component `entity` must have that component.
    #[inline]
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>, access: Access<ComponentId>) -> Self {
        Self { entity, access }
    }

    /// Returns a new instance with a shorter lifetime.
    /// This is useful if you have `&mut FilteredEntityMut`, but you need `FilteredEntityMut`.
    pub fn reborrow(&mut self) -> FilteredEntityMut<'_> {
        // SAFETY: We have exclusive access to the entire entity and its components.
        unsafe { Self::new(self.entity, self.access.clone()) }
    }

    /// Gets read-only access to all of the entity's components.
    #[inline]
    pub fn as_readonly(&self) -> FilteredEntityRef<'_> {
        FilteredEntityRef::from(self)
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
    pub fn access(&self) -> &Access<ComponentId> {
        &self.access
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
    ///     [`Self::contains_type_id`].
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
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_mut<T: Component>(&mut self) -> Option<Mut<'_, T>> {
        let id = self.entity.world().components().get_id(TypeId::of::<T>())?;
        self.access
            .has_component_write(id)
            // SAFETY: We have write access
            .then(|| unsafe { self.entity.get_mut() })
            .flatten()
    }

    /// Consumes self and gets mutable access to the component of type `T`
    /// with the world `'w` lifetime for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn into_mut<T: Component>(self) -> Option<Mut<'w, T>> {
        let id = self.entity.world().components().get_id(TypeId::of::<T>())?;
        self.access
            .has_component_write(id)
            // SAFETY: We have write access
            .then(|| unsafe { self.entity.get_mut() })
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
        self.access
            .has_component_write(component_id)
            // SAFETY: We have write access
            .then(|| unsafe { self.entity.get_mut_by_id(component_id) })
            .flatten()
    }
}

impl<'a> From<EntityMut<'a>> for FilteredEntityMut<'a> {
    fn from(entity: EntityMut<'a>) -> Self {
        // SAFETY:
        // - `EntityMut` guarantees exclusive access to all components in the new `FilteredEntityMut`.
        unsafe {
            let mut access = Access::default();
            access.read_all();
            access.write_all();
            FilteredEntityMut::new(entity.0, access)
        }
    }
}

impl<'a> From<&'a mut EntityMut<'_>> for FilteredEntityMut<'a> {
    fn from(entity: &'a mut EntityMut<'_>) -> Self {
        // SAFETY:
        // - `EntityMut` guarantees exclusive access to all components in the new `FilteredEntityMut`.
        unsafe {
            let mut access = Access::default();
            access.read_all();
            access.write_all();
            FilteredEntityMut::new(entity.0, access)
        }
    }
}

impl<'a> From<EntityWorldMut<'a>> for FilteredEntityMut<'a> {
    fn from(entity: EntityWorldMut<'a>) -> Self {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe {
            let mut access = Access::default();
            access.read_all();
            access.write_all();
            FilteredEntityMut::new(entity.into_unsafe_entity_cell(), access)
        }
    }
}

impl<'a> From<&'a mut EntityWorldMut<'_>> for FilteredEntityMut<'a> {
    fn from(entity: &'a mut EntityWorldMut<'_>) -> Self {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe {
            let mut access = Access::default();
            access.read_all();
            access.write_all();
            FilteredEntityMut::new(entity.as_unsafe_entity_cell(), access)
        }
    }
}

#[derive(Error, Display, Debug)]
pub enum TryFromFilteredError {
    #[display(
        "Conversion failed, filtered entity ref does not have read access to all components"
    )]
    MissingReadAllAccess,

    #[display(
        "Conversion failed, filtered entity ref does not have write access to all components"
    )]
    MissingWriteAllAccess,
}

/// Provides read-only access to a single entity and all its components, save
/// for an explicitly-enumerated set.
#[derive(Clone)]
pub struct EntityRefExcept<'w, B>
where
    B: Bundle,
{
    entity: UnsafeEntityCell<'w>,
    phantom: PhantomData<B>,
}

impl<'w, B> EntityRefExcept<'w, B>
where
    B: Bundle,
{
    /// # Safety
    /// Other users of `UnsafeEntityCell` must only have mutable access to the components in `B`.
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>) -> Self {
        Self {
            entity,
            phantom: PhantomData,
        }
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity.id()
    }

    /// Gets access to the component of type `C` for the current entity. Returns
    /// `None` if the component doesn't have a component of that type or if the
    /// type is one of the excluded components.
    #[inline]
    pub fn get<C>(&self) -> Option<&'w C>
    where
        C: Component,
    {
        let components = self.entity.world().components();
        let id = components.component_id::<C>()?;
        if bundle_contains_component::<B>(components, id) {
            None
        } else {
            // SAFETY: We have read access for all components that weren't
            // covered by the `contains` check above.
            unsafe { self.entity.get() }
        }
    }

    /// Gets access to the component of type `C` for the current entity,
    /// including change detection information. Returns `None` if the component
    /// doesn't have a component of that type or if the type is one of the
    /// excluded components.
    #[inline]
    pub fn get_ref<C>(&self) -> Option<Ref<'w, C>>
    where
        C: Component,
    {
        let components = self.entity.world().components();
        let id = components.component_id::<C>()?;
        if bundle_contains_component::<B>(components, id) {
            None
        } else {
            // SAFETY: We have read access for all components that weren't
            // covered by the `contains` check above.
            unsafe { self.entity.get_ref() }
        }
    }
}

impl<'a, B> From<&'a EntityMutExcept<'_, B>> for EntityRefExcept<'a, B>
where
    B: Bundle,
{
    fn from(entity_mut: &'a EntityMutExcept<'_, B>) -> Self {
        // SAFETY: All accesses that `EntityRefExcept` provides are also
        // accesses that `EntityMutExcept` provides.
        unsafe { EntityRefExcept::new(entity_mut.entity) }
    }
}

/// Provides mutable access to all components of an entity, with the exception
/// of an explicit set.
///
/// This is a rather niche type that should only be used if you need access to
/// *all* components of an entity, while still allowing you to consult other
/// queries that might match entities that this query also matches. If you don't
/// need access to all components, prefer a standard query with a
/// [`crate::query::Without`] filter.
#[derive(Clone)]
pub struct EntityMutExcept<'w, B>
where
    B: Bundle,
{
    entity: UnsafeEntityCell<'w>,
    phantom: PhantomData<B>,
}

impl<'w, B> EntityMutExcept<'w, B>
where
    B: Bundle,
{
    /// # Safety
    /// Other users of `UnsafeEntityCell` must not have access to any components not in `B`.
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>) -> Self {
        Self {
            entity,
            phantom: PhantomData,
        }
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity.id()
    }

    /// Returns a new instance with a shorter lifetime.
    ///
    /// This is useful if you have `&mut EntityMutExcept`, but you need
    /// `EntityMutExcept`.
    pub fn reborrow(&mut self) -> EntityMutExcept<'_, B> {
        // SAFETY: We have exclusive access to the entire entity and the
        // applicable components.
        unsafe { Self::new(self.entity) }
    }

    /// Gets read-only access to all of the entity's components, except for the
    /// ones in `CL`.
    #[inline]
    pub fn as_readonly(&self) -> EntityRefExcept<'_, B> {
        EntityRefExcept::from(self)
    }

    /// Gets access to the component of type `C` for the current entity. Returns
    /// `None` if the component doesn't have a component of that type or if the
    /// type is one of the excluded components.
    #[inline]
    pub fn get<C>(&self) -> Option<&'_ C>
    where
        C: Component,
    {
        self.as_readonly().get()
    }

    /// Gets access to the component of type `C` for the current entity,
    /// including change detection information. Returns `None` if the component
    /// doesn't have a component of that type or if the type is one of the
    /// excluded components.
    #[inline]
    pub fn get_ref<C>(&self) -> Option<Ref<'_, C>>
    where
        C: Component,
    {
        self.as_readonly().get_ref()
    }

    /// Gets mutable access to the component of type `C` for the current entity.
    /// Returns `None` if the component doesn't have a component of that type or
    /// if the type is one of the excluded components.
    #[inline]
    pub fn get_mut<C>(&mut self) -> Option<Mut<'_, C>>
    where
        C: Component,
    {
        let components = self.entity.world().components();
        let id = components.component_id::<C>()?;
        if bundle_contains_component::<B>(components, id) {
            None
        } else {
            // SAFETY: We have write access for all components that weren't
            // covered by the `contains` check above.
            unsafe { self.entity.get_mut() }
        }
    }
}

fn bundle_contains_component<B>(components: &Components, query_id: ComponentId) -> bool
where
    B: Bundle,
{
    let mut found = false;
    B::get_component_ids(components, &mut |maybe_id| {
        if let Some(id) = maybe_id {
            found = found || id == query_id;
        }
    });
    found
}

/// Inserts a dynamic [`Bundle`] into the entity.
///
/// # Safety
///
/// - [`OwningPtr`] and [`StorageType`] iterators must correspond to the
///     [`BundleInfo`] used to construct [`BundleInserter`]
/// - [`Entity`] must correspond to [`EntityLocation`]
#[track_caller]
unsafe fn insert_dynamic_bundle<
    'a,
    I: Iterator<Item = OwningPtr<'a>>,
    S: Iterator<Item = StorageType>,
>(
    mut bundle_inserter: BundleInserter<'_>,
    entity: Entity,
    location: EntityLocation,
    components: I,
    storage_types: S,
) -> EntityLocation {
    struct DynamicInsertBundle<'a, I: Iterator<Item = (StorageType, OwningPtr<'a>)>> {
        components: I,
    }

    impl<'a, I: Iterator<Item = (StorageType, OwningPtr<'a>)>> DynamicBundle
        for DynamicInsertBundle<'a, I>
    {
        fn get_components(self, func: &mut impl FnMut(StorageType, OwningPtr<'_>)) {
            self.components.for_each(|(t, ptr)| func(t, ptr));
        }
    }

    let bundle = DynamicInsertBundle {
        components: storage_types.zip(components),
    };

    // SAFETY: location matches current entity.
    unsafe {
        bundle_inserter.insert(
            entity,
            location,
            bundle,
            InsertMode::Replace,
            #[cfg(feature = "track_change_detection")]
            core::panic::Location::caller(),
        )
    }
}

/// Removes a bundle from the given archetype and returns the resulting archetype (or None if the
/// removal was invalid). in the event that adding the given bundle does not result in an Archetype
/// change. Results are cached in the Archetype Graph to avoid redundant work.
/// if `intersection` is false, attempting to remove a bundle with components _not_ contained in the
/// current archetype will fail, returning None. if `intersection` is true, components in the bundle
/// but not in the current archetype will be ignored
///
/// # Safety
/// `archetype_id` must exist and components in `bundle_info` must exist
unsafe fn remove_bundle_from_archetype(
    archetypes: &mut Archetypes,
    storages: &mut Storages,
    components: &Components,
    observers: &Observers,
    archetype_id: ArchetypeId,
    bundle_info: &BundleInfo,
    intersection: bool,
) -> Option<ArchetypeId> {
    // check the archetype graph to see if the Bundle has been removed from this archetype in the
    // past
    let remove_bundle_result = {
        let edges = archetypes[archetype_id].edges();
        if intersection {
            edges.get_remove_bundle(bundle_info.id())
        } else {
            edges.get_take_bundle(bundle_info.id())
        }
    };
    let result = if let Some(result) = remove_bundle_result {
        // this Bundle removal result is cached. just return that!
        result
    } else {
        let mut next_table_components;
        let mut next_sparse_set_components;
        let next_table_id;
        {
            let current_archetype = &mut archetypes[archetype_id];
            let mut removed_table_components = Vec::new();
            let mut removed_sparse_set_components = Vec::new();
            for component_id in bundle_info.iter_explicit_components() {
                if current_archetype.contains(component_id) {
                    // SAFETY: bundle components were already initialized by bundles.get_info
                    let component_info = unsafe { components.get_info_unchecked(component_id) };
                    match component_info.storage_type() {
                        StorageType::Table => removed_table_components.push(component_id),
                        StorageType::SparseSet => removed_sparse_set_components.push(component_id),
                    }
                } else if !intersection {
                    // a component in the bundle was not present in the entity's archetype, so this
                    // removal is invalid cache the result in the archetype
                    // graph
                    current_archetype
                        .edges_mut()
                        .insert_take_bundle(bundle_info.id(), None);
                    return None;
                }
            }

            // sort removed components so we can do an efficient "sorted remove". archetype
            // components are already sorted
            removed_table_components.sort_unstable();
            removed_sparse_set_components.sort_unstable();
            next_table_components = current_archetype.table_components().collect();
            next_sparse_set_components = current_archetype.sparse_set_components().collect();
            sorted_remove(&mut next_table_components, &removed_table_components);
            sorted_remove(
                &mut next_sparse_set_components,
                &removed_sparse_set_components,
            );

            next_table_id = if removed_table_components.is_empty() {
                current_archetype.table_id()
            } else {
                // SAFETY: all components in next_table_components exist
                unsafe {
                    storages
                        .tables
                        .get_id_or_insert(&next_table_components, components)
                }
            };
        }

        let new_archetype_id = archetypes.get_id_or_insert(
            components,
            observers,
            next_table_id,
            next_table_components,
            next_sparse_set_components,
        );
        Some(new_archetype_id)
    };
    let current_archetype = &mut archetypes[archetype_id];
    // cache the result in an edge
    if intersection {
        current_archetype
            .edges_mut()
            .insert_remove_bundle(bundle_info.id(), result);
    } else {
        current_archetype
            .edges_mut()
            .insert_take_bundle(bundle_info.id(), result);
    }
    result
}

fn sorted_remove<T: Eq + Ord + Copy>(source: &mut Vec<T>, remove: &[T]) {
    let mut remove_index = 0;
    source.retain(|value| {
        while remove_index < remove.len() && *value > remove[remove_index] {
            remove_index += 1;
        }

        if remove_index < remove.len() {
            *value != remove[remove_index]
        } else {
            true
        }
    });
}

/// Moves component data out of storage.
///
/// This function leaves the underlying memory unchanged, but the component behind
/// returned pointer is semantically owned by the caller and will not be dropped in its original location.
/// Caller is responsible to drop component data behind returned pointer.
///
/// # Safety
/// - `location.table_row` must be in bounds of column of component id `component_id`
/// - `component_id` must be valid
/// - `components` must come from the same world as `self`
/// - The relevant table row **must be removed** by the caller once all components are taken, without dropping the value
#[inline]
pub(crate) unsafe fn take_component<'a>(
    storages: &'a mut Storages,
    components: &Components,
    removed_components: &mut RemovedComponentEvents,
    component_id: ComponentId,
    entity: Entity,
    location: EntityLocation,
) -> OwningPtr<'a> {
    // SAFETY: caller promises component_id to be valid
    let component_info = unsafe { components.get_info_unchecked(component_id) };
    removed_components.send(component_id, entity);
    match component_info.storage_type() {
        StorageType::Table => {
            let table = &mut storages.tables[location.table_id];
            // SAFETY:
            // - archetypes only store valid table_rows
            // - index is in bounds as promised by caller
            // - promote is safe because the caller promises to remove the table row without dropping it immediately afterwards
            unsafe { table.take_component(component_id, location.table_row) }
        }
        StorageType::SparseSet => storages
            .sparse_sets
            .get_mut(component_id)
            .unwrap()
            .remove_and_forget(entity)
            .unwrap(),
    }
}

/// Types that can be used to fetch components from an entity dynamically by
/// [`ComponentId`]s.
///
/// Provided implementations are:
/// - [`ComponentId`]: Returns a single untyped reference.
/// - `[ComponentId; N]` and `&[ComponentId; N]`: Returns a same-sized array of untyped references.
/// - `&[ComponentId]`: Returns a [`Vec`] of untyped references.
/// - [`&HashSet<ComponentId>`](HashSet): Returns a [`HashMap`] of IDs to untyped references.
///
/// # Performance
///
/// - The slice and array implementations perform an aliased mutability check in
///   [`DynamicComponentFetch::fetch_mut`] that is `O(N^2)`.
/// - The [`HashSet`] implementation performs no such check as the type itself
///   guarantees unique IDs.
/// - The single [`ComponentId`] implementation performs no such check as only
///   one reference is returned.
///
/// # Safety
///
/// Implementor must ensure that:
/// - No aliased mutability is caused by the returned references.
/// - [`DynamicComponentFetch::fetch_ref`] returns only read-only references.
pub unsafe trait DynamicComponentFetch {
    /// The read-only reference type returned by [`DynamicComponentFetch::fetch_ref`].
    type Ref<'w>;

    /// The mutable reference type returned by [`DynamicComponentFetch::fetch_mut`].
    type Mut<'w>;

    /// Returns untyped read-only reference(s) to the component(s) with the
    /// given [`ComponentId`]s, as determined by `self`.
    ///
    /// # Safety
    ///
    /// It is the caller's responsibility to ensure that:
    /// - The given [`UnsafeEntityCell`] has read-only access to the fetched components.
    /// - No other mutable references to the fetched components exist at the same time.
    ///
    /// # Errors
    ///
    /// - Returns [`EntityComponentError::MissingComponent`] if a component is missing from the entity.
    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityComponentError>;

    /// Returns untyped mutable reference(s) to the component(s) with the
    /// given [`ComponentId`]s, as determined by `self`.
    ///
    /// # Safety
    ///
    /// It is the caller's responsibility to ensure that:
    /// - The given [`UnsafeEntityCell`] has mutable access to the fetched components.
    /// - No other references to the fetched components exist at the same time.
    ///
    /// # Errors
    ///
    /// - Returns [`EntityComponentError::MissingComponent`] if a component is missing from the entity.
    /// - Returns [`EntityComponentError::AliasedMutability`] if a component is requested multiple times.
    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityComponentError>;
}

// SAFETY:
// - No aliased mutability is caused because a single reference is returned.
// - No mutable references are returned by `fetch_ref`.
unsafe impl DynamicComponentFetch for ComponentId {
    type Ref<'w> = Ptr<'w>;
    type Mut<'w> = MutUntyped<'w>;

    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityComponentError> {
        // SAFETY: caller ensures that the cell has read access to the component.
        unsafe { cell.get_by_id(self) }.ok_or(EntityComponentError::MissingComponent(self))
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        // SAFETY: caller ensures that the cell has mutable access to the component.
        unsafe { cell.get_mut_by_id(self) }.ok_or(EntityComponentError::MissingComponent(self))
    }
}

// SAFETY:
// - No aliased mutability is caused because the array is checked for duplicates.
// - No mutable references are returned by `fetch_ref`.
unsafe impl<const N: usize> DynamicComponentFetch for [ComponentId; N] {
    type Ref<'w> = [Ptr<'w>; N];
    type Mut<'w> = [MutUntyped<'w>; N];

    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityComponentError> {
        <&Self>::fetch_ref(&self, cell)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        <&Self>::fetch_mut(&self, cell)
    }
}

// SAFETY:
// - No aliased mutability is caused because the array is checked for duplicates.
// - No mutable references are returned by `fetch_ref`.
unsafe impl<const N: usize> DynamicComponentFetch for &'_ [ComponentId; N] {
    type Ref<'w> = [Ptr<'w>; N];
    type Mut<'w> = [MutUntyped<'w>; N];

    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityComponentError> {
        let mut ptrs = [const { MaybeUninit::uninit() }; N];
        for (ptr, &id) in core::iter::zip(&mut ptrs, self) {
            *ptr = MaybeUninit::new(
                // SAFETY: caller ensures that the cell has read access to the component.
                unsafe { cell.get_by_id(id) }.ok_or(EntityComponentError::MissingComponent(id))?,
            );
        }

        // SAFETY: Each ptr was initialized in the loop above.
        let ptrs = ptrs.map(|ptr| unsafe { MaybeUninit::assume_init(ptr) });

        Ok(ptrs)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        // Check for duplicate component IDs.
        for i in 0..self.len() {
            for j in 0..i {
                if self[i] == self[j] {
                    return Err(EntityComponentError::AliasedMutability(self[i]));
                }
            }
        }

        let mut ptrs = [const { MaybeUninit::uninit() }; N];
        for (ptr, &id) in core::iter::zip(&mut ptrs, self) {
            *ptr = MaybeUninit::new(
                // SAFETY: caller ensures that the cell has mutable access to the component.
                unsafe { cell.get_mut_by_id(id) }
                    .ok_or(EntityComponentError::MissingComponent(id))?,
            );
        }

        // SAFETY: Each ptr was initialized in the loop above.
        let ptrs = ptrs.map(|ptr| unsafe { MaybeUninit::assume_init(ptr) });

        Ok(ptrs)
    }
}

// SAFETY:
// - No aliased mutability is caused because the slice is checked for duplicates.
// - No mutable references are returned by `fetch_ref`.
unsafe impl DynamicComponentFetch for &'_ [ComponentId] {
    type Ref<'w> = Vec<Ptr<'w>>;
    type Mut<'w> = Vec<MutUntyped<'w>>;

    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityComponentError> {
        let mut ptrs = Vec::with_capacity(self.len());
        for &id in self {
            ptrs.push(
                // SAFETY: caller ensures that the cell has read access to the component.
                unsafe { cell.get_by_id(id) }.ok_or(EntityComponentError::MissingComponent(id))?,
            );
        }
        Ok(ptrs)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        // Check for duplicate component IDs.
        for i in 0..self.len() {
            for j in 0..i {
                if self[i] == self[j] {
                    return Err(EntityComponentError::AliasedMutability(self[i]));
                }
            }
        }

        let mut ptrs = Vec::with_capacity(self.len());
        for &id in self {
            ptrs.push(
                // SAFETY: caller ensures that the cell has mutable access to the component.
                unsafe { cell.get_mut_by_id(id) }
                    .ok_or(EntityComponentError::MissingComponent(id))?,
            );
        }
        Ok(ptrs)
    }
}

// SAFETY:
// - No aliased mutability is caused because `HashSet` guarantees unique elements.
// - No mutable references are returned by `fetch_ref`.
unsafe impl DynamicComponentFetch for &'_ HashSet<ComponentId> {
    type Ref<'w> = HashMap<ComponentId, Ptr<'w>>;
    type Mut<'w> = HashMap<ComponentId, MutUntyped<'w>>;

    unsafe fn fetch_ref(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Ref<'_>, EntityComponentError> {
        let mut ptrs = HashMap::with_capacity(self.len());
        for &id in self {
            ptrs.insert(
                id,
                // SAFETY: caller ensures that the cell has read access to the component.
                unsafe { cell.get_by_id(id) }.ok_or(EntityComponentError::MissingComponent(id))?,
            );
        }
        Ok(ptrs)
    }

    unsafe fn fetch_mut(
        self,
        cell: UnsafeEntityCell<'_>,
    ) -> Result<Self::Mut<'_>, EntityComponentError> {
        let mut ptrs = HashMap::with_capacity(self.len());
        for &id in self {
            ptrs.insert(
                id,
                // SAFETY: caller ensures that the cell has mutable access to the component.
                unsafe { cell.get_mut_by_id(id) }
                    .ok_or(EntityComponentError::MissingComponent(id))?,
            );
        }
        Ok(ptrs)
    }
}

#[cfg(test)]
mod tests {
    use bevy_ptr::{OwningPtr, Ptr};
    use core::panic::AssertUnwindSafe;

    use crate::{
        self as bevy_ecs,
        change_detection::MutUntyped,
        component::ComponentId,
        prelude::*,
        system::{assert_is_system, RunSystemOnce as _},
        world::{error::EntityComponentError, FilteredEntityMut, FilteredEntityRef},
    };

    use super::{EntityMutExcept, EntityRefExcept};

    #[test]
    fn sorted_remove() {
        let mut a = vec![1, 2, 3, 4, 5, 6, 7];
        let b = vec![1, 2, 3, 5, 7];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![4, 6]);

        let mut a = vec![1];
        let b = vec![1];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![]);

        let mut a = vec![1];
        let b = vec![2];
        super::sorted_remove(&mut a, &b);

        assert_eq!(a, vec![1]);
    }

    #[derive(Component, Clone, Copy, Debug, PartialEq)]
    struct TestComponent(u32);

    #[derive(Component, Clone, Copy, Debug, PartialEq)]
    #[component(storage = "SparseSet")]
    struct TestComponent2(u32);

    #[test]
    fn entity_ref_get_by_id() {
        let mut world = World::new();
        let entity = world.spawn(TestComponent(42)).id();
        let component_id = world
            .components()
            .get_id(core::any::TypeId::of::<TestComponent>())
            .unwrap();

        let entity = world.entity(entity);
        let test_component = entity.get_by_id(component_id).unwrap();
        // SAFETY: points to a valid `TestComponent`
        let test_component = unsafe { test_component.deref::<TestComponent>() };

        assert_eq!(test_component.0, 42);
    }

    #[test]
    fn entity_mut_get_by_id() {
        let mut world = World::new();
        let entity = world.spawn(TestComponent(42)).id();
        let component_id = world
            .components()
            .get_id(core::any::TypeId::of::<TestComponent>())
            .unwrap();

        let mut entity_mut = world.entity_mut(entity);
        let mut test_component = entity_mut.get_mut_by_id(component_id).unwrap();
        {
            test_component.set_changed();
            let test_component =
                // SAFETY: `test_component` has unique access of the `EntityWorldMut` and is not used afterwards
                unsafe { test_component.into_inner().deref_mut::<TestComponent>() };
            test_component.0 = 43;
        }

        let entity = world.entity(entity);
        let test_component = entity.get_by_id(component_id).unwrap();
        // SAFETY: `TestComponent` is the correct component type
        let test_component = unsafe { test_component.deref::<TestComponent>() };

        assert_eq!(test_component.0, 43);
    }

    #[test]
    fn entity_ref_get_by_id_invalid_component_id() {
        let invalid_component_id = ComponentId::new(usize::MAX);

        let mut world = World::new();
        let entity = world.spawn_empty().id();
        let entity = world.entity(entity);
        assert!(entity.get_by_id(invalid_component_id).is_err());
    }

    #[test]
    fn entity_mut_get_by_id_invalid_component_id() {
        let invalid_component_id = ComponentId::new(usize::MAX);

        let mut world = World::new();
        let mut entity = world.spawn_empty();
        assert!(entity.get_by_id(invalid_component_id).is_err());
        assert!(entity.get_mut_by_id(invalid_component_id).is_err());
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7387
    #[test]
    fn entity_mut_world_scope_panic() {
        let mut world = World::new();

        let mut entity = world.spawn_empty();
        let old_location = entity.location();
        let id = entity.id();
        let res = std::panic::catch_unwind(AssertUnwindSafe(|| {
            entity.world_scope(|w| {
                // Change the entity's `EntityLocation`, which invalidates the original `EntityWorldMut`.
                // This will get updated at the end of the scope.
                w.entity_mut(id).insert(TestComponent(0));

                // Ensure that the entity location still gets updated even in case of a panic.
                panic!("this should get caught by the outer scope")
            });
        }));
        assert!(res.is_err());

        // Ensure that the location has been properly updated.
        assert_ne!(entity.location(), old_location);
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn removing_sparse_updates_archetype_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn((Dense(0), Sparse)).id();
        let e2 = world.spawn((Dense(1), Sparse)).id();

        world.entity_mut(e1).remove::<Sparse>();
        assert_eq!(world.entity(e2).get::<Dense>().unwrap(), &Dense(1));
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn removing_dense_updates_table_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn((Dense(0), Sparse)).id();
        let e2 = world.spawn((Dense(1), Sparse)).id();

        world.entity_mut(e1).remove::<Dense>();
        assert_eq!(world.entity(e2).get::<Dense>().unwrap(), &Dense(1));
    }

    // Test that calling retain with `()` removes all components.
    #[test]
    fn retain_nothing() {
        #[derive(Component)]
        struct Marker<const N: usize>;

        let mut world = World::new();
        let ent = world.spawn((Marker::<1>, Marker::<2>, Marker::<3>)).id();

        world.entity_mut(ent).retain::<()>();
        assert_eq!(world.entity(ent).archetype().components().next(), None);
    }

    // Test removing some components with `retain`, including components not on the entity.
    #[test]
    fn retain_some_components() {
        #[derive(Component)]
        struct Marker<const N: usize>;

        let mut world = World::new();
        let ent = world.spawn((Marker::<1>, Marker::<2>, Marker::<3>)).id();

        world.entity_mut(ent).retain::<(Marker<2>, Marker<4>)>();
        // Check that marker 2 was retained.
        assert!(world.entity(ent).get::<Marker<2>>().is_some());
        // Check that only marker 2 was retained.
        assert_eq!(
            world
                .entity(ent)
                .archetype()
                .components()
                .collect::<Vec<_>>()
                .len(),
            1
        );
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn inserting_sparse_updates_archetype_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn(Dense(0)).id();
        let e2 = world.spawn(Dense(1)).id();

        world.entity_mut(e1).insert(Sparse);
        assert_eq!(world.entity(e2).get::<Dense>().unwrap(), &Dense(1));
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn inserting_dense_updates_archetype_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        struct Dense2;

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn(Dense(0)).id();
        let e2 = world.spawn(Dense(1)).id();

        world.entity_mut(e1).insert(Sparse).remove::<Sparse>();

        // archetype with [e2, e1]
        // table with [e1, e2]

        world.entity_mut(e2).insert(Dense2);

        assert_eq!(world.entity(e1).get::<Dense>().unwrap(), &Dense(0));
    }

    #[test]
    fn inserting_dense_updates_table_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        struct Dense2;

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn(Dense(0)).id();
        let e2 = world.spawn(Dense(1)).id();

        world.entity_mut(e1).insert(Sparse).remove::<Sparse>();

        // archetype with [e2, e1]
        // table with [e1, e2]

        world.entity_mut(e1).insert(Dense2);

        assert_eq!(world.entity(e2).get::<Dense>().unwrap(), &Dense(1));
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn despawning_entity_updates_archetype_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn(Dense(0)).id();
        let e2 = world.spawn(Dense(1)).id();

        world.entity_mut(e1).insert(Sparse).remove::<Sparse>();

        // archetype with [e2, e1]
        // table with [e1, e2]

        world.entity_mut(e2).despawn();

        assert_eq!(world.entity(e1).get::<Dense>().unwrap(), &Dense(0));
    }

    // regression test for https://github.com/bevyengine/bevy/pull/7805
    #[test]
    fn despawning_entity_updates_table_row() {
        #[derive(Component, PartialEq, Debug)]
        struct Dense(u8);

        #[derive(Component)]
        #[component(storage = "SparseSet")]
        struct Sparse;

        let mut world = World::new();
        let e1 = world.spawn(Dense(0)).id();
        let e2 = world.spawn(Dense(1)).id();

        world.entity_mut(e1).insert(Sparse).remove::<Sparse>();

        // archetype with [e2, e1]
        // table with [e1, e2]

        world.entity_mut(e1).despawn();

        assert_eq!(world.entity(e2).get::<Dense>().unwrap(), &Dense(1));
    }

    #[test]
    fn entity_mut_insert_by_id() {
        let mut world = World::new();
        let test_component_id = world.register_component::<TestComponent>();

        let mut entity = world.spawn_empty();
        OwningPtr::make(TestComponent(42), |ptr| {
            // SAFETY: `ptr` matches the component id
            unsafe { entity.insert_by_id(test_component_id, ptr) };
        });

        let components: Vec<_> = world.query::<&TestComponent>().iter(&world).collect();

        assert_eq!(components, vec![&TestComponent(42)]);

        // Compare with `insert_bundle_by_id`

        let mut entity = world.spawn_empty();
        OwningPtr::make(TestComponent(84), |ptr| {
            // SAFETY: `ptr` matches the component id
            unsafe { entity.insert_by_ids(&[test_component_id], vec![ptr].into_iter()) };
        });

        let components: Vec<_> = world.query::<&TestComponent>().iter(&world).collect();

        assert_eq!(components, vec![&TestComponent(42), &TestComponent(84)]);
    }

    #[test]
    fn entity_mut_insert_bundle_by_id() {
        let mut world = World::new();
        let test_component_id = world.register_component::<TestComponent>();
        let test_component_2_id = world.register_component::<TestComponent2>();

        let component_ids = [test_component_id, test_component_2_id];
        let test_component_value = TestComponent(42);
        let test_component_2_value = TestComponent2(84);

        let mut entity = world.spawn_empty();
        OwningPtr::make(test_component_value, |ptr1| {
            OwningPtr::make(test_component_2_value, |ptr2| {
                // SAFETY: `ptr1` and `ptr2` match the component ids
                unsafe { entity.insert_by_ids(&component_ids, vec![ptr1, ptr2].into_iter()) };
            });
        });

        let dynamic_components: Vec<_> = world
            .query::<(&TestComponent, &TestComponent2)>()
            .iter(&world)
            .collect();

        assert_eq!(
            dynamic_components,
            vec![(&TestComponent(42), &TestComponent2(84))]
        );

        // Compare with `World` generated using static type equivalents
        let mut static_world = World::new();

        static_world.spawn((test_component_value, test_component_2_value));
        let static_components: Vec<_> = static_world
            .query::<(&TestComponent, &TestComponent2)>()
            .iter(&static_world)
            .collect();

        assert_eq!(dynamic_components, static_components);
    }

    #[test]
    fn entity_mut_remove_by_id() {
        let mut world = World::new();
        let test_component_id = world.register_component::<TestComponent>();

        let mut entity = world.spawn(TestComponent(42));
        entity.remove_by_id(test_component_id);

        let components: Vec<_> = world.query::<&TestComponent>().iter(&world).collect();

        assert_eq!(components, vec![] as Vec<&TestComponent>);

        // remove non-existent component does not panic
        world.spawn_empty().remove_by_id(test_component_id);
    }

    /// Tests that components can be accessed through an `EntityRefExcept`.
    #[test]
    fn entity_ref_except() {
        let mut world = World::new();
        world.register_component::<TestComponent>();
        world.register_component::<TestComponent2>();

        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        let mut query = world.query::<EntityRefExcept<TestComponent>>();

        let mut found = false;
        for entity_ref in query.iter_mut(&mut world) {
            found = true;
            assert!(entity_ref.get::<TestComponent>().is_none());
            assert!(entity_ref.get_ref::<TestComponent>().is_none());
            assert!(matches!(
                entity_ref.get::<TestComponent2>(),
                Some(TestComponent2(0))
            ));
        }

        assert!(found);
    }

    // Test that a single query can't both contain a mutable reference to a
    // component C and an `EntityRefExcept` that doesn't include C among its
    // exclusions.
    #[test]
    #[should_panic]
    fn entity_ref_except_conflicts_with_self() {
        let mut world = World::new();
        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        // This should panic, because we have a mutable borrow on
        // `TestComponent` but have a simultaneous indirect immutable borrow on
        // that component via `EntityRefExcept`.
        world.run_system_once(system).unwrap();

        fn system(_: Query<(&mut TestComponent, EntityRefExcept<TestComponent2>)>) {}
    }

    // Test that an `EntityRefExcept` that doesn't include a component C among
    // its exclusions can't coexist with a mutable query for that component.
    #[test]
    #[should_panic]
    fn entity_ref_except_conflicts_with_other() {
        let mut world = World::new();
        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        // This should panic, because we have a mutable borrow on
        // `TestComponent` but have a simultaneous indirect immutable borrow on
        // that component via `EntityRefExcept`.
        world.run_system_once(system).unwrap();

        fn system(_: Query<&mut TestComponent>, _: Query<EntityRefExcept<TestComponent2>>) {}
    }

    // Test that an `EntityRefExcept` with an exception for some component C can
    // coexist with a query for that component C.
    #[test]
    fn entity_ref_except_doesnt_conflict() {
        let mut world = World::new();
        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        world.run_system_once(system).unwrap();

        fn system(_: Query<&mut TestComponent>, query: Query<EntityRefExcept<TestComponent>>) {
            for entity_ref in query.iter() {
                assert!(matches!(
                    entity_ref.get::<TestComponent2>(),
                    Some(TestComponent2(0))
                ));
            }
        }
    }

    /// Tests that components can be mutably accessed through an
    /// `EntityMutExcept`.
    #[test]
    fn entity_mut_except() {
        let mut world = World::new();
        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        let mut query = world.query::<EntityMutExcept<TestComponent>>();

        let mut found = false;
        for mut entity_mut in query.iter_mut(&mut world) {
            found = true;
            assert!(entity_mut.get::<TestComponent>().is_none());
            assert!(entity_mut.get_ref::<TestComponent>().is_none());
            assert!(entity_mut.get_mut::<TestComponent>().is_none());
            assert!(matches!(
                entity_mut.get::<TestComponent2>(),
                Some(TestComponent2(0))
            ));
        }

        assert!(found);
    }

    // Test that a single query can't both contain a mutable reference to a
    // component C and an `EntityMutExcept` that doesn't include C among its
    // exclusions.
    #[test]
    #[should_panic]
    fn entity_mut_except_conflicts_with_self() {
        let mut world = World::new();
        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        // This should panic, because we have a mutable borrow on
        // `TestComponent` but have a simultaneous indirect immutable borrow on
        // that component via `EntityRefExcept`.
        world.run_system_once(system).unwrap();

        fn system(_: Query<(&mut TestComponent, EntityMutExcept<TestComponent2>)>) {}
    }

    // Test that an `EntityMutExcept` that doesn't include a component C among
    // its exclusions can't coexist with a query for that component.
    #[test]
    #[should_panic]
    fn entity_mut_except_conflicts_with_other() {
        let mut world = World::new();
        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        // This should panic, because we have a mutable borrow on
        // `TestComponent` but have a simultaneous indirect immutable borrow on
        // that component via `EntityRefExcept`.
        world.run_system_once(system).unwrap();

        fn system(_: Query<&mut TestComponent>, mut query: Query<EntityMutExcept<TestComponent2>>) {
            for mut entity_mut in query.iter_mut() {
                assert!(entity_mut
                    .get_mut::<TestComponent2>()
                    .is_some_and(|component| component.0 == 0));
            }
        }
    }

    // Test that an `EntityMutExcept` with an exception for some component C can
    // coexist with a query for that component C.
    #[test]
    fn entity_mut_except_doesnt_conflict() {
        let mut world = World::new();
        world.spawn(TestComponent(0)).insert(TestComponent2(0));

        world.run_system_once(system).unwrap();

        fn system(_: Query<&mut TestComponent>, mut query: Query<EntityMutExcept<TestComponent>>) {
            for mut entity_mut in query.iter_mut() {
                assert!(entity_mut
                    .get_mut::<TestComponent2>()
                    .is_some_and(|component| component.0 == 0));
            }
        }
    }

    #[derive(Component)]
    struct A;

    #[derive(Resource)]
    struct R;

    #[test]
    fn disjoint_access() {
        fn disjoint_readonly(_: Query<EntityMut, With<A>>, _: Query<EntityRef, Without<A>>) {}

        fn disjoint_mutable(_: Query<EntityMut, With<A>>, _: Query<EntityMut, Without<A>>) {}

        assert_is_system(disjoint_readonly);
        assert_is_system(disjoint_mutable);
    }

    #[test]
    fn ref_compatible() {
        fn borrow_system(_: Query<(EntityRef, &A)>, _: Query<&A>) {}

        assert_is_system(borrow_system);
    }

    #[test]
    fn ref_compatible_with_resource() {
        fn borrow_system(_: Query<EntityRef>, _: Res<R>) {}

        assert_is_system(borrow_system);
    }

    #[test]
    #[ignore] // This should pass, but it currently fails due to limitations in our access model.
    fn ref_compatible_with_resource_mut() {
        fn borrow_system(_: Query<EntityRef>, _: ResMut<R>) {}

        assert_is_system(borrow_system);
    }

    #[test]
    #[should_panic]
    fn ref_incompatible_with_mutable_component() {
        fn incompatible_system(_: Query<(EntityRef, &mut A)>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    #[should_panic]
    fn ref_incompatible_with_mutable_query() {
        fn incompatible_system(_: Query<EntityRef>, _: Query<&mut A>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    fn mut_compatible_with_entity() {
        fn borrow_mut_system(_: Query<(Entity, EntityMut)>) {}

        assert_is_system(borrow_mut_system);
    }

    #[test]
    #[ignore] // This should pass, but it currently fails due to limitations in our access model.
    fn mut_compatible_with_resource() {
        fn borrow_mut_system(_: Res<R>, _: Query<EntityMut>) {}

        assert_is_system(borrow_mut_system);
    }

    #[test]
    #[ignore] // This should pass, but it currently fails due to limitations in our access model.
    fn mut_compatible_with_resource_mut() {
        fn borrow_mut_system(_: ResMut<R>, _: Query<EntityMut>) {}

        assert_is_system(borrow_mut_system);
    }

    #[test]
    #[should_panic]
    fn mut_incompatible_with_read_only_component() {
        fn incompatible_system(_: Query<(EntityMut, &A)>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    #[should_panic]
    fn mut_incompatible_with_mutable_component() {
        fn incompatible_system(_: Query<(EntityMut, &mut A)>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    #[should_panic]
    fn mut_incompatible_with_read_only_query() {
        fn incompatible_system(_: Query<EntityMut>, _: Query<&A>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    #[should_panic]
    fn mut_incompatible_with_mutable_query() {
        fn incompatible_system(_: Query<EntityMut>, _: Query<&mut A>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    fn filtered_entity_ref_normal() {
        let mut world = World::new();
        let a_id = world.register_component::<A>();

        let e: FilteredEntityRef = world.spawn(A).into();

        assert!(e.get::<A>().is_some());
        assert!(e.get_ref::<A>().is_some());
        assert!(e.get_change_ticks::<A>().is_some());
        assert!(e.get_by_id(a_id).is_some());
        assert!(e.get_change_ticks_by_id(a_id).is_some());
    }

    #[test]
    fn filtered_entity_ref_missing() {
        let mut world = World::new();
        let a_id = world.register_component::<A>();

        let e: FilteredEntityRef = world.spawn(()).into();

        assert!(e.get::<A>().is_none());
        assert!(e.get_ref::<A>().is_none());
        assert!(e.get_change_ticks::<A>().is_none());
        assert!(e.get_by_id(a_id).is_none());
        assert!(e.get_change_ticks_by_id(a_id).is_none());
    }

    #[test]
    fn filtered_entity_mut_normal() {
        let mut world = World::new();
        let a_id = world.register_component::<A>();

        let mut e: FilteredEntityMut = world.spawn(A).into();

        assert!(e.get::<A>().is_some());
        assert!(e.get_ref::<A>().is_some());
        assert!(e.get_mut::<A>().is_some());
        assert!(e.get_change_ticks::<A>().is_some());
        assert!(e.get_by_id(a_id).is_some());
        assert!(e.get_mut_by_id(a_id).is_some());
        assert!(e.get_change_ticks_by_id(a_id).is_some());
    }

    #[test]
    fn filtered_entity_mut_missing() {
        let mut world = World::new();
        let a_id = world.register_component::<A>();

        let mut e: FilteredEntityMut = world.spawn(()).into();

        assert!(e.get::<A>().is_none());
        assert!(e.get_ref::<A>().is_none());
        assert!(e.get_mut::<A>().is_none());
        assert!(e.get_change_ticks::<A>().is_none());
        assert!(e.get_by_id(a_id).is_none());
        assert!(e.get_mut_by_id(a_id).is_none());
        assert!(e.get_change_ticks_by_id(a_id).is_none());
    }

    #[derive(Component, PartialEq, Eq, Debug)]
    struct X(usize);

    #[derive(Component, PartialEq, Eq, Debug)]
    struct Y(usize);

    #[test]
    fn get_components() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();
        let e2 = world.spawn(X(8)).id();
        let e3 = world.spawn_empty().id();

        assert_eq!(
            Some((&X(7), &Y(10))),
            world.entity(e1).get_components::<(&X, &Y)>()
        );
        assert_eq!(None, world.entity(e2).get_components::<(&X, &Y)>());
        assert_eq!(None, world.entity(e3).get_components::<(&X, &Y)>());
    }

    #[test]
    fn get_by_id_array() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();
        let e2 = world.spawn(X(8)).id();
        let e3 = world.spawn_empty().id();

        let x_id = world.register_component::<X>();
        let y_id = world.register_component::<Y>();

        assert_eq!(
            Ok((&X(7), &Y(10))),
            world
                .entity(e1)
                .get_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(y_id)),
            world
                .entity(e2)
                .get_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(x_id)),
            world
                .entity(e3)
                .get_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
    }

    #[test]
    fn get_by_id_vec() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();
        let e2 = world.spawn(X(8)).id();
        let e3 = world.spawn_empty().id();

        let x_id = world.register_component::<X>();
        let y_id = world.register_component::<Y>();

        assert_eq!(
            Ok((&X(7), &Y(10))),
            world
                .entity(e1)
                .get_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[Ptr; 2], _> = ptrs.try_into() else {
                        panic!("get_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(y_id)),
            world
                .entity(e2)
                .get_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[Ptr; 2], _> = ptrs.try_into() else {
                        panic!("get_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(x_id)),
            world
                .entity(e3)
                .get_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[Ptr; 2], _> = ptrs.try_into() else {
                        panic!("get_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.deref::<X>() }, unsafe { y_ptr.deref::<Y>() })
                })
        );
    }

    #[test]
    fn get_mut_by_id_array() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();
        let e2 = world.spawn(X(8)).id();
        let e3 = world.spawn_empty().id();

        let x_id = world.register_component::<X>();
        let y_id = world.register_component::<Y>();

        assert_eq!(
            Ok((&mut X(7), &mut Y(10))),
            world
                .entity_mut(e1)
                .get_mut_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(y_id)),
            world
                .entity_mut(e2)
                .get_mut_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(x_id)),
            world
                .entity_mut(e3)
                .get_mut_by_id([x_id, y_id])
                .map(|[x_ptr, y_ptr]| {
                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );

        assert_eq!(
            Err(EntityComponentError::AliasedMutability(x_id)),
            world
                .entity_mut(e1)
                .get_mut_by_id([x_id, x_id])
                .map(|_| { unreachable!() })
        );
        assert_eq!(
            Err(EntityComponentError::AliasedMutability(x_id)),
            world
                .entity_mut(e3)
                .get_mut_by_id([x_id, x_id])
                .map(|_| { unreachable!() })
        );
    }

    #[test]
    fn get_mut_by_id_vec() {
        let mut world = World::default();
        let e1 = world.spawn((X(7), Y(10))).id();
        let e2 = world.spawn(X(8)).id();
        let e3 = world.spawn_empty().id();

        let x_id = world.register_component::<X>();
        let y_id = world.register_component::<Y>();

        assert_eq!(
            Ok((&mut X(7), &mut Y(10))),
            world
                .entity_mut(e1)
                .get_mut_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[MutUntyped; 2], _> = ptrs.try_into() else {
                        panic!("get_mut_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(y_id)),
            world
                .entity_mut(e2)
                .get_mut_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[MutUntyped; 2], _> = ptrs.try_into() else {
                        panic!("get_mut_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );
        assert_eq!(
            Err(EntityComponentError::MissingComponent(x_id)),
            world
                .entity_mut(e3)
                .get_mut_by_id(&[x_id, y_id] as &[ComponentId])
                .map(|ptrs| {
                    let Ok([x_ptr, y_ptr]): Result<[MutUntyped; 2], _> = ptrs.try_into() else {
                        panic!("get_mut_by_id(slice) didn't return 2 elements")
                    };

                    // SAFETY: components match the id they were fetched with
                    (unsafe { x_ptr.into_inner().deref_mut::<X>() }, unsafe {
                        y_ptr.into_inner().deref_mut::<Y>()
                    })
                })
        );

        assert_eq!(
            Err(EntityComponentError::AliasedMutability(x_id)),
            world
                .entity_mut(e1)
                .get_mut_by_id(&[x_id, x_id])
                .map(|_| { unreachable!() })
        );
        assert_eq!(
            Err(EntityComponentError::AliasedMutability(x_id)),
            world
                .entity_mut(e3)
                .get_mut_by_id(&[x_id, x_id])
                .map(|_| { unreachable!() })
        );
    }
}
