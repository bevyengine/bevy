use crate::{
    archetype::Archetype,
    change_detection::{ComponentTicks, MaybeLocation, Tick},
    component::{Component, ComponentId},
    entity::{ContainsEntity, Entity, EntityEquivalent, EntityLocation},
    world::{
        error::EntityComponentError, unsafe_world_cell::UnsafeEntityCell, AccessScope, All,
        DynamicComponentFetch, Ref,
    },
};

use core::{
    any::TypeId,
    cmp::Ordering,
    hash::{Hash, Hasher},
};

/// Provides read-only access to a single [`Entity`] and the components allowed
/// by the [`AccessScope`] `S`. Plain `EntityRef`s have an [`AccessScope`] of
/// [`All`], providing access to all components of the entity.
///
/// # [`AccessScope`]s
///
/// Access scopes describe what you can access on an `EntityRef`. The default
/// scope is [`All`], which provides access to all components of the entity.
/// Other scopes, such as [`Filtered`] and [`Except`], can restrict access to
/// only a subset of components.
///
/// See the documentation of [`AccessScope`] for more details.
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
///
/// [`Filtered`]: crate::world::Filtered
/// [`Except`]: crate::world::Except
#[derive(Copy, Clone)]
pub struct EntityRef<'w, S: AccessScope = All> {
    pub(super) cell: UnsafeEntityCell<'w>,
    scope: S,
}

impl<'w, S: AccessScope> EntityRef<'w, S> {
    /// # Safety
    ///
    /// Caller must ensure `scope` does not exceed the read permissions of `cell`
    /// in a way that would violate Rust's aliasing rules, including simultaneous
    /// access of `cell` via an `EntityMut` or any other means.
    #[inline]
    pub(crate) unsafe fn new(cell: UnsafeEntityCell<'w>, scope: S) -> Self {
        Self { cell, scope }
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
    pub fn get<T: Component>(&self) -> Option<&'w T> {
        // SAFETY: `self` was constructed with a `scope` that doesn't violate
        // Rust's aliasing rules for `cell`.
        unsafe { self.cell.get::<T>(&self.scope) }
    }

    /// Gets access to the component of type `T` for the current entity,
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    #[inline]
    pub fn get_ref<T: Component>(&self) -> Option<Ref<'w, T>> {
        // SAFETY: `self` was constructed with a `scope` that doesn't violate
        // Rust's aliasing rules for `cell`.
        unsafe { self.cell.get_ref::<T>(&self.scope) }
    }

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    #[inline]
    pub fn get_change_ticks<T: Component>(&self) -> Option<ComponentTicks> {
        // SAFETY: `self` was constructed with a `scope` that doesn't violate
        // Rust's aliasing rules for `cell`.
        unsafe { self.cell.get_change_ticks::<T>(&self.scope) }
    }

    /// Retrieves the change ticks for the given [`ComponentId`]. This can be useful for implementing change
    /// detection in custom runtimes.
    ///
    /// **You should prefer to use the typed API [`EntityRef::get_change_ticks`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    #[inline]
    pub fn get_change_ticks_by_id(&self, component_id: ComponentId) -> Option<ComponentTicks> {
        // SAFETY: `self` was constructed with a `scope` that doesn't violate
        // Rust's aliasing rules for `cell`.
        unsafe { self.cell.get_change_ticks_by_id(&self.scope, component_id) }
    }

    /// Returns untyped read-only reference(s) to component(s) for the
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
        // SAFETY: `self` was constructed with a `scope` that doesn't violate
        // Rust's aliasing rules for `cell`.
        unsafe { component_ids.fetch_ref(self.cell, &self.scope) }
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

impl<S: AccessScope> PartialEq for EntityRef<'_, S> {
    fn eq(&self, other: &Self) -> bool {
        self.entity() == other.entity()
    }
}

impl<S: AccessScope> Eq for EntityRef<'_, S> {}

impl<S: AccessScope> PartialOrd for EntityRef<'_, S> {
    /// [`EntityRef`]'s comparison trait implementations match the underlying [`Entity`],
    /// and cannot discern between different worlds.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<S: AccessScope> Ord for EntityRef<'_, S> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.entity().cmp(&other.entity())
    }
}

impl<S: AccessScope> Hash for EntityRef<'_, S> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity().hash(state);
    }
}

impl<S: AccessScope> ContainsEntity for EntityRef<'_, S> {
    fn entity(&self) -> Entity {
        self.id()
    }
}

// SAFETY: This type represents one Entity. We implement the comparison traits based on that Entity.
unsafe impl<S: AccessScope> EntityEquivalent for EntityRef<'_, S> {}
