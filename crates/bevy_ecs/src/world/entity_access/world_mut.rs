use crate::{
    archetype::Archetype,
    bundle::{
        Bundle, BundleFromComponents, BundleInserter, BundleRemover, DynamicBundle, InsertMode,
    },
    change_detection::{ComponentTicks, MaybeLocation, MutUntyped, Tick},
    component::{Component, ComponentId, Components, Mutable, StorageType},
    entity::{Entity, EntityCloner, EntityClonerBuilder, EntityLocation, OptIn, OptOut},
    event::{EntityComponentsTrigger, IntoTargetEvent, TargetEvent},
    lifecycle::{Despawn, Remove, Replace, DESPAWN, REMOVE, REPLACE},
    observer::Observer,
    query::{Access, DebugCheckedUnwrap, ReadOnlyQueryData, ReleaseStateQueryData},
    relationship::RelationshipHookMode,
    resource::Resource,
    storage::{SparseSets, Table},
    system::IntoObserverSystem,
    world::{
        error::EntityComponentError, unsafe_world_cell::UnsafeEntityCell, ComponentEntry,
        DynamicComponentFetch, EntityMut, EntityRef, FilteredEntityMut, FilteredEntityRef, Mut,
        OccupiedComponentEntry, Ref, VacantComponentEntry, World,
    },
};

use alloc::vec::Vec;
use bevy_ptr::{move_as_ptr, MovingPtr, OwningPtr};
use core::{any::TypeId, marker::PhantomData, mem::MaybeUninit};

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
    location: Option<EntityLocation>,
}

impl<'w> EntityWorldMut<'w> {
    #[track_caller]
    #[inline(never)]
    #[cold]
    fn panic_despawned(&self) -> ! {
        panic!(
            "Entity {} {}",
            self.entity,
            self.world.entities().get_spawned(self.entity).unwrap_err()
        );
    }

    #[inline(always)]
    #[track_caller]
    pub(crate) fn assert_not_despawned(&self) {
        if self.location.is_none() {
            self.panic_despawned()
        }
    }

    #[inline(always)]
    fn as_unsafe_entity_cell_readonly(&self) -> UnsafeEntityCell<'_> {
        let location = self.location();
        let last_change_tick = self.world.last_change_tick;
        let change_tick = self.world.read_change_tick();
        UnsafeEntityCell::new(
            self.world.as_unsafe_world_cell_readonly(),
            self.entity,
            location,
            last_change_tick,
            change_tick,
        )
    }

    #[inline(always)]
    fn as_unsafe_entity_cell(&mut self) -> UnsafeEntityCell<'_> {
        let location = self.location();
        let last_change_tick = self.world.last_change_tick;
        let change_tick = self.world.change_tick();
        UnsafeEntityCell::new(
            self.world.as_unsafe_world_cell(),
            self.entity,
            location,
            last_change_tick,
            change_tick,
        )
    }

    #[inline(always)]
    fn into_unsafe_entity_cell(self) -> UnsafeEntityCell<'w> {
        let location = self.location();
        let last_change_tick = self.world.last_change_tick;
        let change_tick = self.world.change_tick();
        UnsafeEntityCell::new(
            self.world.as_unsafe_world_cell(),
            self.entity,
            location,
            last_change_tick,
            change_tick,
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
        location: Option<EntityLocation>,
    ) -> Self {
        debug_assert!(world.entities().contains(entity));
        debug_assert_eq!(world.entities().get(entity).unwrap(), location);

        EntityWorldMut {
            world,
            entity,
            location,
        }
    }

    /// Consumes `self` and returns read-only access to all of the entity's
    /// components, with the world `'w` lifetime.
    pub fn into_readonly(self) -> EntityRef<'w> {
        EntityRef::from(self)
    }

    /// Gets read-only access to all of the entity's components.
    #[inline]
    pub fn as_readonly(&self) -> EntityRef<'_> {
        EntityRef::from(self)
    }

    /// Consumes `self` and returns non-structural mutable access to all of the
    /// entity's components, with the world `'w` lifetime.
    pub fn into_mutable(self) -> EntityMut<'w> {
        EntityMut::from(self)
    }

    /// Gets non-structural mutable access to all of the entity's components.
    #[inline]
    pub fn as_mutable(&mut self) -> EntityMut<'_> {
        EntityMut::from(self)
    }

    /// Returns the [ID](Entity) of the current entity.
    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(&self) -> Entity {
        self.entity
    }

    /// Gets metadata indicating the location where the current entity is stored.
    #[inline]
    pub fn try_location(&self) -> Option<EntityLocation> {
        self.location
    }

    /// Returns if the entity is spawned or not.
    #[inline]
    pub fn is_spawned(&self) -> bool {
        self.try_location().is_some()
    }

    /// Returns the archetype that the current entity belongs to.
    #[inline]
    pub fn try_archetype(&self) -> Option<&Archetype> {
        self.try_location()
            .map(|location| &self.world.archetypes[location.archetype_id])
    }

    /// Gets metadata indicating the location where the current entity is stored.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn location(&self) -> EntityLocation {
        match self.try_location() {
            Some(a) => a,
            None => self.panic_despawned(),
        }
    }

    /// Returns the archetype that the current entity belongs to.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn archetype(&self) -> &Archetype {
        match self.try_archetype() {
            Some(a) => a,
            None => self.panic_despawned(),
        }
    }

    /// Returns `true` if the current entity has a component of type `T`.
    /// Otherwise, this returns `false`.
    ///
    /// ## Notes
    ///
    /// If you do not know the concrete type of a component, consider using
    /// [`Self::contains_id`] or [`Self::contains_type_id`].
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
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
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
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
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn contains_type_id(&self, type_id: TypeId) -> bool {
        self.as_unsafe_entity_cell_readonly()
            .contains_type_id(type_id)
    }

    /// Gets access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn get<T: Component>(&self) -> Option<&'_ T> {
        self.as_readonly().get()
    }

    /// Returns read-only components for the current entity that match the query `Q`.
    ///
    /// # Panics
    ///
    /// If the entity does not have the components required by the query `Q` or if the entity
    /// has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn components<Q: ReadOnlyQueryData + ReleaseStateQueryData>(&self) -> Q::Item<'_, 'static> {
        self.as_readonly().components::<Q>()
    }

    /// Returns read-only components for the current entity that match the query `Q`,
    /// or `None` if the entity does not have the components required by the query `Q`.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn get_components<Q: ReadOnlyQueryData + ReleaseStateQueryData>(
        &self,
    ) -> Option<Q::Item<'_, 'static>> {
        self.as_readonly().get_components::<Q>()
    }

    /// Returns components for the current entity that match the query `Q`,
    /// or `None` if the entity does not have the components required by the query `Q`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Component)]
    /// struct X(usize);
    /// #[derive(Component)]
    /// struct Y(usize);
    ///
    /// # let mut world = World::default();
    /// let mut entity = world.spawn((X(0), Y(0)));
    /// // Get mutable access to two components at once
    /// // SAFETY: X and Y are different components
    /// let (mut x, mut y) =
    ///     unsafe { entity.get_components_mut_unchecked::<(&mut X, &mut Y)>() }.unwrap();
    /// *x = X(1);
    /// *y = Y(1);
    /// // This would trigger undefined behavior, as the `&mut X`s would alias:
    /// // entity.get_components_mut_unchecked::<(&mut X, &mut X)>();
    /// ```
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that
    /// the `QueryData` does not provide aliasing mutable references to the same component.
    pub unsafe fn get_components_mut_unchecked<Q: ReleaseStateQueryData>(
        &mut self,
    ) -> Option<Q::Item<'_, 'static>> {
        // SAFETY: Caller the `QueryData` does not provide aliasing mutable references to the same component
        unsafe { self.as_mutable().into_components_mut_unchecked::<Q>() }
    }

    /// Consumes self and returns components for the current entity that match the query `Q` for the world lifetime `'w`,
    /// or `None` if the entity does not have the components required by the query `Q`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Component)]
    /// struct X(usize);
    /// #[derive(Component)]
    /// struct Y(usize);
    ///
    /// # let mut world = World::default();
    /// let mut entity = world.spawn((X(0), Y(0)));
    /// // Get mutable access to two components at once
    /// // SAFETY: X and Y are different components
    /// let (mut x, mut y) =
    ///     unsafe { entity.into_components_mut_unchecked::<(&mut X, &mut Y)>() }.unwrap();
    /// *x = X(1);
    /// *y = Y(1);
    /// // This would trigger undefined behavior, as the `&mut X`s would alias:
    /// // entity.into_components_mut_unchecked::<(&mut X, &mut X)>();
    /// ```
    ///
    /// # Safety
    /// It is the caller's responsibility to ensure that
    /// the `QueryData` does not provide aliasing mutable references to the same component.
    pub unsafe fn into_components_mut_unchecked<Q: ReleaseStateQueryData>(
        self,
    ) -> Option<Q::Item<'w, 'static>> {
        // SAFETY: Caller the `QueryData` does not provide aliasing mutable references to the same component
        unsafe { self.into_mutable().into_components_mut_unchecked::<Q>() }
    }

    /// Consumes `self` and gets access to the component of type `T` with
    /// the world `'w` lifetime for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn into_borrow<T: Component>(self) -> Option<&'w T> {
        self.into_readonly().get()
    }

    /// Gets access to the component of type `T` for the current entity,
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn get_ref<T: Component>(&self) -> Option<Ref<'_, T>> {
        self.as_readonly().get_ref()
    }

    /// Consumes `self` and gets access to the component of type `T`
    /// with the world `'w` lifetime for the current entity,
    /// including change detection information as a [`Ref`].
    ///
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn into_ref<T: Component>(self) -> Option<Ref<'w, T>> {
        self.into_readonly().get_ref()
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn get_mut<T: Component<Mutability = Mutable>>(&mut self) -> Option<Mut<'_, T>> {
        self.as_mutable().into_mut()
    }

    /// Temporarily removes a [`Component`] `T` from this [`Entity`] and runs the
    /// provided closure on it, returning the result if `T` was available.
    /// This will trigger the `Remove` and `Replace` component hooks without
    /// causing an archetype move.
    ///
    /// This is most useful with immutable components, where removal and reinsertion
    /// is the only way to modify a value.
    ///
    /// If you do not need to ensure the above hooks are triggered, and your component
    /// is mutable, prefer using [`get_mut`](EntityWorldMut::get_mut).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # use bevy_ecs::prelude::*;
    /// #
    /// #[derive(Component, PartialEq, Eq, Debug)]
    /// #[component(immutable)]
    /// struct Foo(bool);
    ///
    /// # let mut world = World::default();
    /// # world.register_component::<Foo>();
    /// #
    /// # let entity = world.spawn(Foo(false)).id();
    /// #
    /// # let mut entity = world.entity_mut(entity);
    /// #
    /// # assert_eq!(entity.get::<Foo>(), Some(&Foo(false)));
    /// #
    /// entity.modify_component(|foo: &mut Foo| {
    ///     foo.0 = true;
    /// });
    /// #
    /// # assert_eq!(entity.get::<Foo>(), Some(&Foo(true)));
    /// ```
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn modify_component<T: Component, R>(&mut self, f: impl FnOnce(&mut T) -> R) -> Option<R> {
        self.assert_not_despawned();

        let result = self
            .world
            .modify_component(self.entity, f)
            .expect("entity access must be valid")?;

        self.update_location();

        Some(result)
    }

    /// Temporarily removes a [`Component`] `T` from this [`Entity`] and runs the
    /// provided closure on it, returning the result if `T` was available.
    /// This will trigger the `Remove` and `Replace` component hooks without
    /// causing an archetype move.
    ///
    /// This is most useful with immutable components, where removal and reinsertion
    /// is the only way to modify a value.
    ///
    /// If you do not need to ensure the above hooks are triggered, and your component
    /// is mutable, prefer using [`get_mut`](EntityWorldMut::get_mut).
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn modify_component_by_id<R>(
        &mut self,
        component_id: ComponentId,
        f: impl for<'a> FnOnce(MutUntyped<'a>) -> R,
    ) -> Option<R> {
        self.assert_not_despawned();

        let result = self
            .world
            .modify_component_by_id(self.entity, component_id, f)
            .expect("entity access must be valid")?;

        self.update_location();

        Some(result)
    }

    /// Gets mutable access to the component of type `T` for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Safety
    ///
    /// - `T` must be a mutable component
    #[inline]
    pub unsafe fn get_mut_assume_mutable<T: Component>(&mut self) -> Option<Mut<'_, T>> {
        self.as_mutable().into_mut_assume_mutable()
    }

    /// Consumes `self` and gets mutable access to the component of type `T`
    /// with the world `'w` lifetime for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn into_mut<T: Component<Mutability = Mutable>>(self) -> Option<Mut<'w, T>> {
        // SAFETY: consuming `self` implies exclusive access
        unsafe { self.into_unsafe_entity_cell().get_mut() }
    }

    /// Consumes `self` and gets mutable access to the component of type `T`
    /// with the world `'w` lifetime for the current entity.
    /// Returns `None` if the entity does not have a component of type `T`.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    ///
    /// # Safety
    ///
    /// - `T` must be a mutable component
    #[inline]
    pub unsafe fn into_mut_assume_mutable<T: Component>(self) -> Option<Mut<'w, T>> {
        // SAFETY: consuming `self` implies exclusive access
        unsafe { self.into_unsafe_entity_cell().get_mut_assume_mutable() }
    }

    /// Gets a reference to the resource of the given type
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist.
    /// Use [`get_resource`](EntityWorldMut::get_resource) instead if you want to handle this case.
    #[inline]
    #[track_caller]
    pub fn resource<R: Resource>(&self) -> &R {
        self.world.resource::<R>()
    }

    /// Gets a mutable reference to the resource of the given type
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist.
    /// Use [`get_resource_mut`](World::get_resource_mut) instead if you want to handle this case.
    ///
    /// If you want to instead insert a value if the resource does not exist,
    /// use [`get_resource_or_insert_with`](World::get_resource_or_insert_with).
    #[inline]
    #[track_caller]
    pub fn resource_mut<R: Resource>(&mut self) -> Mut<'_, R> {
        self.world.resource_mut::<R>()
    }

    /// Gets a reference to the resource of the given type if it exists
    #[inline]
    pub fn get_resource<R: Resource>(&self) -> Option<&R> {
        self.world.get_resource()
    }

    /// Gets a mutable reference to the resource of the given type if it exists
    #[inline]
    pub fn get_resource_mut<R: Resource>(&mut self) -> Option<Mut<'_, R>> {
        self.world.get_resource_mut()
    }

    /// Temporarily removes the requested resource from the [`World`], runs custom user code,
    /// then re-adds the resource before returning.
    ///
    /// # Panics
    ///
    /// Panics if the resource does not exist.
    /// Use [`try_resource_scope`](Self::try_resource_scope) instead if you want to handle this case.
    ///
    /// See [`World::resource_scope`] for further details.
    #[track_caller]
    pub fn resource_scope<R: Resource, U>(
        &mut self,
        f: impl FnOnce(&mut EntityWorldMut, Mut<R>) -> U,
    ) -> U {
        let id = self.id();
        self.world_scope(|world| {
            world.resource_scope(|world, res| {
                // Acquiring a new EntityWorldMut here and using that instead of `self` is fine because
                // the outer `world_scope` will handle updating our location if it gets changed by the user code
                let mut this = world.entity_mut(id);
                f(&mut this, res)
            })
        })
    }

    /// Temporarily removes the requested resource from the [`World`] if it exists, runs custom user code,
    /// then re-adds the resource before returning. Returns `None` if the resource does not exist in the [`World`].
    ///
    /// See [`World::try_resource_scope`] for further details.
    pub fn try_resource_scope<R: Resource, U>(
        &mut self,
        f: impl FnOnce(&mut EntityWorldMut, Mut<R>) -> U,
    ) -> Option<U> {
        let id = self.id();
        self.world_scope(|world| {
            world.try_resource_scope(|world, res| {
                // Acquiring a new EntityWorldMut here and using that instead of `self` is fine because
                // the outer `world_scope` will handle updating our location if it gets changed by the user code
                let mut this = world.entity_mut(id);
                f(&mut this, res)
            })
        })
    }

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
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
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn get_change_ticks_by_id(&self, component_id: ComponentId) -> Option<ComponentTicks> {
        self.as_readonly().get_change_ticks_by_id(component_id)
    }

    /// Returns untyped read-only reference(s) to component(s) for the
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
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
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
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn into_borrow_by_id<F: DynamicComponentFetch>(
        self,
        component_ids: F,
    ) -> Result<F::Ref<'w>, EntityComponentError> {
        self.into_readonly().get_by_id(component_ids)
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
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn get_mut_by_id<F: DynamicComponentFetch>(
        &mut self,
        component_ids: F,
    ) -> Result<F::Mut<'_>, EntityComponentError> {
        self.as_mutable().into_mut_by_id(component_ids)
    }

    /// Returns [untyped mutable reference(s)](MutUntyped) to component(s) for
    /// the current entity, based on the given [`ComponentId`]s.
    /// Assumes the given [`ComponentId`]s refer to mutable components.
    ///
    /// **You should prefer to use the typed API [`EntityWorldMut::get_mut_assume_mutable`] where
    /// possible and only use this in cases where the actual component types
    /// are not known at compile time.**
    ///
    /// Unlike [`EntityWorldMut::get_mut_assume_mutable`], this returns untyped reference(s) to
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
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the provided [`ComponentId`]s must refer to mutable components.
    #[inline]
    pub unsafe fn get_mut_assume_mutable_by_id<F: DynamicComponentFetch>(
        &mut self,
        component_ids: F,
    ) -> Result<F::Mut<'_>, EntityComponentError> {
        self.as_mutable()
            .into_mut_assume_mutable_by_id(component_ids)
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
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[inline]
    pub fn into_mut_by_id<F: DynamicComponentFetch>(
        self,
        component_ids: F,
    ) -> Result<F::Mut<'w>, EntityComponentError> {
        self.into_mutable().into_mut_by_id(component_ids)
    }

    /// Consumes `self` and returns [untyped mutable reference(s)](MutUntyped)
    /// to component(s) with lifetime `'w` for the current entity, based on the
    /// given [`ComponentId`]s.
    /// Assumes the given [`ComponentId`]s refer to mutable components.
    ///
    /// **You should prefer to use the typed API [`EntityWorldMut::into_mut_assume_mutable`] where
    /// possible and only use this in cases where the actual component types
    /// are not known at compile time.**
    ///
    /// Unlike [`EntityWorldMut::into_mut_assume_mutable`], this returns untyped reference(s) to
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
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the provided [`ComponentId`]s must refer to mutable components.
    #[inline]
    pub unsafe fn into_mut_assume_mutable_by_id<F: DynamicComponentFetch>(
        self,
        component_ids: F,
    ) -> Result<F::Mut<'w>, EntityComponentError> {
        self.into_mutable()
            .into_mut_assume_mutable_by_id(component_ids)
    }

    /// Adds a [`Bundle`] of components to the entity.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[track_caller]
    pub fn insert<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        move_as_ptr!(bundle);
        self.insert_with_caller(
            bundle,
            InsertMode::Replace,
            MaybeLocation::caller(),
            RelationshipHookMode::Run,
        )
    }

    /// Adds a [`Bundle`] of components to the entity.
    /// [`Relationship`](crate::relationship::Relationship) components in the bundle will follow the configuration
    /// in `relationship_hook_mode`.
    ///
    /// This will overwrite any previous value(s) of the same component type.
    ///
    /// # Warning
    ///
    /// This can easily break the integrity of relationships. This is intended to be used for cloning and spawning code internals,
    /// not most user-facing scenarios.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[track_caller]
    pub fn insert_with_relationship_hook_mode<T: Bundle>(
        &mut self,
        bundle: T,
        relationship_hook_mode: RelationshipHookMode,
    ) -> &mut Self {
        move_as_ptr!(bundle);
        self.insert_with_caller(
            bundle,
            InsertMode::Replace,
            MaybeLocation::caller(),
            relationship_hook_mode,
        )
    }

    /// Adds a [`Bundle`] of components to the entity without overwriting.
    ///
    /// This will leave any previous value(s) of the same component type
    /// unchanged.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[track_caller]
    pub fn insert_if_new<T: Bundle>(&mut self, bundle: T) -> &mut Self {
        move_as_ptr!(bundle);
        self.insert_with_caller(
            bundle,
            InsertMode::Keep,
            MaybeLocation::caller(),
            RelationshipHookMode::Run,
        )
    }

    /// Adds a [`Bundle`] of components to the entity.
    #[inline]
    pub(crate) fn insert_with_caller<T: Bundle>(
        &mut self,
        bundle: MovingPtr<'_, T>,
        mode: InsertMode,
        caller: MaybeLocation,
        relationship_hook_mode: RelationshipHookMode,
    ) -> &mut Self {
        let location = self.location();
        let change_tick = self.world.change_tick();
        let mut bundle_inserter =
            BundleInserter::new::<T>(self.world, location.archetype_id, change_tick);
        // SAFETY:
        // - `location` matches current entity and thus must currently exist in the source
        //   archetype for this inserter and its location within the archetype.
        // - `T` matches the type used to create the `BundleInserter`.
        // - `apply_effect` is called exactly once after this function.
        // - The value pointed at by `bundle` is not accessed for anything other than `apply_effect`
        //   and the caller ensures that the value is not accessed or dropped after this function
        //   returns.
        let (bundle, location) = bundle.partial_move(|bundle| unsafe {
            bundle_inserter.insert(
                self.entity,
                location,
                bundle,
                mode,
                caller,
                relationship_hook_mode,
            )
        });
        self.location = Some(location);
        self.world.flush();
        self.update_location();
        // SAFETY:
        // - This is called exactly once after the `BundleInsert::insert` call before returning to safe code.
        // - `bundle` points to the same `B` that `BundleInsert::insert` was called on.
        unsafe { T::apply_effect(bundle, self) };
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
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[track_caller]
    pub unsafe fn insert_by_id(
        &mut self,
        component_id: ComponentId,
        component: OwningPtr<'_>,
    ) -> &mut Self {
        self.insert_by_id_with_caller(
            component_id,
            component,
            InsertMode::Replace,
            MaybeLocation::caller(),
            RelationshipHookMode::Run,
        )
    }

    /// # Safety
    ///
    /// - [`ComponentId`] must be from the same world as [`EntityWorldMut`]
    /// - [`OwningPtr`] must be a valid reference to the type represented by [`ComponentId`]
    #[inline]
    pub(crate) unsafe fn insert_by_id_with_caller(
        &mut self,
        component_id: ComponentId,
        component: OwningPtr<'_>,
        mode: InsertMode,
        caller: MaybeLocation,
        relationship_hook_insert_mode: RelationshipHookMode,
    ) -> &mut Self {
        let location = self.location();
        let change_tick = self.world.change_tick();
        let bundle_id = self.world.bundles.init_component_info(
            &mut self.world.storages,
            &self.world.components,
            component_id,
        );
        let storage_type = self.world.bundles.get_storage_unchecked(bundle_id);

        let bundle_inserter =
            BundleInserter::new_with_id(self.world, location.archetype_id, bundle_id, change_tick);

        self.location = Some(insert_dynamic_bundle(
            bundle_inserter,
            self.entity,
            location,
            Some(component).into_iter(),
            Some(storage_type).iter().cloned(),
            mode,
            caller,
            relationship_hook_insert_mode,
        ));
        self.world.flush();
        self.update_location();
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
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[track_caller]
    pub unsafe fn insert_by_ids<'a, I: Iterator<Item = OwningPtr<'a>>>(
        &mut self,
        component_ids: &[ComponentId],
        iter_components: I,
    ) -> &mut Self {
        self.insert_by_ids_internal(component_ids, iter_components, RelationshipHookMode::Run)
    }

    #[track_caller]
    pub(crate) unsafe fn insert_by_ids_internal<'a, I: Iterator<Item = OwningPtr<'a>>>(
        &mut self,
        component_ids: &[ComponentId],
        iter_components: I,
        relationship_hook_insert_mode: RelationshipHookMode,
    ) -> &mut Self {
        let location = self.location();
        let change_tick = self.world.change_tick();
        let bundle_id = self.world.bundles.init_dynamic_info(
            &mut self.world.storages,
            &self.world.components,
            component_ids,
        );
        let mut storage_types =
            core::mem::take(self.world.bundles.get_storages_unchecked(bundle_id));
        let bundle_inserter =
            BundleInserter::new_with_id(self.world, location.archetype_id, bundle_id, change_tick);

        self.location = Some(insert_dynamic_bundle(
            bundle_inserter,
            self.entity,
            location,
            iter_components,
            (*storage_types).iter().cloned(),
            InsertMode::Replace,
            MaybeLocation::caller(),
            relationship_hook_insert_mode,
        ));
        *self.world.bundles.get_storages_unchecked(bundle_id) = core::mem::take(&mut storage_types);
        self.world.flush();
        self.update_location();
        self
    }

    /// Removes all components in the [`Bundle`] from the entity and returns their previous values.
    ///
    /// **Note:** If the entity does not have every component in the bundle, this method will not
    /// remove any of them.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[must_use]
    #[track_caller]
    pub fn take<T: Bundle + BundleFromComponents>(&mut self) -> Option<T> {
        let location = self.location();
        let entity = self.entity;

        let mut remover =
            // SAFETY: The archetype id must be valid since this entity is in it.
            unsafe { BundleRemover::new::<T>(self.world, location.archetype_id, true) }?;
        // SAFETY: The passed location has the sane archetype as the remover, since they came from the same location.
        let (new_location, result) = unsafe {
            remover.remove(
                entity,
                location,
                MaybeLocation::caller(),
                |sets, table, components, bundle_components| {
                    let mut bundle_components = bundle_components.iter().copied();
                    (
                        false,
                        T::from_components(&mut (sets, table), &mut |(sets, table)| {
                            let component_id = bundle_components.next().unwrap();
                            // SAFETY: the component existed to be removed, so its id must be valid.
                            let component_info = components.get_info_unchecked(component_id);
                            match component_info.storage_type() {
                                StorageType::Table => {
                                    table
                                        .as_mut()
                                        // SAFETY: The table must be valid if the component is in it.
                                        .debug_checked_unwrap()
                                        // SAFETY: The remover is cleaning this up.
                                        .take_component(component_id, location.table_row)
                                }
                                StorageType::SparseSet => sets
                                    .get_mut(component_id)
                                    .unwrap()
                                    .remove_and_forget(entity)
                                    .unwrap(),
                            }
                        }),
                    )
                },
            )
        };
        self.location = Some(new_location);

        self.world.flush();
        self.update_location();
        Some(result)
    }

    /// Removes any components in the [`Bundle`] from the entity.
    ///
    /// See [`EntityCommands::remove`](crate::system::EntityCommands::remove) for more details.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[track_caller]
    pub fn remove<T: Bundle>(&mut self) -> &mut Self {
        self.remove_with_caller::<T>(MaybeLocation::caller())
    }

    #[inline]
    pub(crate) fn remove_with_caller<T: Bundle>(&mut self, caller: MaybeLocation) -> &mut Self {
        let location = self.location();

        let Some(mut remover) =
            // SAFETY: The archetype id must be valid since this entity is in it.
            (unsafe { BundleRemover::new::<T>(self.world, location.archetype_id, false) })
        else {
            return self;
        };
        // SAFETY: The remover archetype came from the passed location and the removal can not fail.
        let new_location = unsafe {
            remover.remove(
                self.entity,
                location,
                caller,
                BundleRemover::empty_pre_remove,
            )
        }
        .0;

        self.location = Some(new_location);
        self.world.flush();
        self.update_location();
        self
    }

    /// Removes all components in the [`Bundle`] and remove all required components for each component in the bundle
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[track_caller]
    pub fn remove_with_requires<T: Bundle>(&mut self) -> &mut Self {
        self.remove_with_requires_with_caller::<T>(MaybeLocation::caller())
    }

    pub(crate) fn remove_with_requires_with_caller<T: Bundle>(
        &mut self,
        caller: MaybeLocation,
    ) -> &mut Self {
        let location = self.location();
        let bundle_id = self.world.register_contributed_bundle_info::<T>();

        // SAFETY: We just created the bundle, and the archetype is valid, since we are in it.
        let Some(mut remover) = (unsafe {
            BundleRemover::new_with_id(self.world, location.archetype_id, bundle_id, false)
        }) else {
            return self;
        };
        // SAFETY: The remover archetype came from the passed location and the removal can not fail.
        let new_location = unsafe {
            remover.remove(
                self.entity,
                location,
                caller,
                BundleRemover::empty_pre_remove,
            )
        }
        .0;

        self.location = Some(new_location);
        self.world.flush();
        self.update_location();
        self
    }

    /// Removes any components except those in the [`Bundle`] (and its Required Components) from the entity.
    ///
    /// See [`EntityCommands::retain`](crate::system::EntityCommands::retain) for more details.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[track_caller]
    pub fn retain<T: Bundle>(&mut self) -> &mut Self {
        self.retain_with_caller::<T>(MaybeLocation::caller())
    }

    #[inline]
    pub(crate) fn retain_with_caller<T: Bundle>(&mut self, caller: MaybeLocation) -> &mut Self {
        let old_location = self.location();
        let retained_bundle = self.world.register_bundle_info::<T>();
        let archetypes = &mut self.world.archetypes;

        // SAFETY: `retained_bundle` exists as we just registered it.
        let retained_bundle_info = unsafe { self.world.bundles.get_unchecked(retained_bundle) };
        let old_archetype = &mut archetypes[old_location.archetype_id];

        // PERF: this could be stored in an Archetype Edge
        let to_remove = &old_archetype
            .iter_components()
            .filter(|c| !retained_bundle_info.contributed_components().contains(c))
            .collect::<Vec<_>>();
        let remove_bundle = self.world.bundles.init_dynamic_info(
            &mut self.world.storages,
            &self.world.components,
            to_remove,
        );

        // SAFETY: We just created the bundle, and the archetype is valid, since we are in it.
        let Some(mut remover) = (unsafe {
            BundleRemover::new_with_id(self.world, old_location.archetype_id, remove_bundle, false)
        }) else {
            return self;
        };
        // SAFETY: The remover archetype came from the passed location and the removal can not fail.
        let new_location = unsafe {
            remover.remove(
                self.entity,
                old_location,
                caller,
                BundleRemover::empty_pre_remove,
            )
        }
        .0;

        self.location = Some(new_location);
        self.world.flush();
        self.update_location();
        self
    }

    /// Removes a dynamic [`Component`] from the entity if it exists.
    ///
    /// You should prefer to use the typed API [`EntityWorldMut::remove`] where possible.
    ///
    /// # Panics
    ///
    /// Panics if the provided [`ComponentId`] does not exist in the [`World`] or if the
    /// entity has been despawned while this `EntityWorldMut` is still alive.
    #[track_caller]
    pub fn remove_by_id(&mut self, component_id: ComponentId) -> &mut Self {
        self.remove_by_id_with_caller(component_id, MaybeLocation::caller())
    }

    #[inline]
    pub(crate) fn remove_by_id_with_caller(
        &mut self,
        component_id: ComponentId,
        caller: MaybeLocation,
    ) -> &mut Self {
        let location = self.location();
        let components = &mut self.world.components;

        let bundle_id = self.world.bundles.init_component_info(
            &mut self.world.storages,
            components,
            component_id,
        );

        // SAFETY: We just created the bundle, and the archetype is valid, since we are in it.
        let Some(mut remover) = (unsafe {
            BundleRemover::new_with_id(self.world, location.archetype_id, bundle_id, false)
        }) else {
            return self;
        };
        // SAFETY: The remover archetype came from the passed location and the removal can not fail.
        let new_location = unsafe {
            remover.remove(
                self.entity,
                location,
                caller,
                BundleRemover::empty_pre_remove,
            )
        }
        .0;

        self.location = Some(new_location);
        self.world.flush();
        self.update_location();
        self
    }

    /// Removes a dynamic bundle from the entity if it exists.
    ///
    /// You should prefer to use the typed API [`EntityWorldMut::remove`] where possible.
    ///
    /// # Panics
    ///
    /// Panics if any of the provided [`ComponentId`]s do not exist in the [`World`] or if the
    /// entity has been despawned while this `EntityWorldMut` is still alive.
    #[track_caller]
    pub fn remove_by_ids(&mut self, component_ids: &[ComponentId]) -> &mut Self {
        self.remove_by_ids_with_caller(
            component_ids,
            MaybeLocation::caller(),
            RelationshipHookMode::Run,
            BundleRemover::empty_pre_remove,
        )
    }

    #[inline]
    pub(crate) fn remove_by_ids_with_caller<T: 'static>(
        &mut self,
        component_ids: &[ComponentId],
        caller: MaybeLocation,
        relationship_hook_mode: RelationshipHookMode,
        pre_remove: impl FnOnce(
            &mut SparseSets,
            Option<&mut Table>,
            &Components,
            &[ComponentId],
        ) -> (bool, T),
    ) -> &mut Self {
        let location = self.location();
        let components = &mut self.world.components;

        let bundle_id = self.world.bundles.init_dynamic_info(
            &mut self.world.storages,
            components,
            component_ids,
        );

        // SAFETY: We just created the bundle, and the archetype is valid, since we are in it.
        let Some(mut remover) = (unsafe {
            BundleRemover::new_with_id(self.world, location.archetype_id, bundle_id, false)
        }) else {
            return self;
        };
        remover.relationship_hook_mode = relationship_hook_mode;
        // SAFETY: The remover archetype came from the passed location and the removal can not fail.
        let new_location = unsafe { remover.remove(self.entity, location, caller, pre_remove) }.0;

        self.location = Some(new_location);
        self.world.flush();
        self.update_location();
        self
    }

    /// Removes all components associated with the entity.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    #[track_caller]
    pub fn clear(&mut self) -> &mut Self {
        self.clear_with_caller(MaybeLocation::caller())
    }

    #[inline]
    pub(crate) fn clear_with_caller(&mut self, caller: MaybeLocation) -> &mut Self {
        let location = self.location();
        // PERF: this should not be necessary
        let component_ids: Vec<ComponentId> = self.archetype().components().to_vec();
        let components = &mut self.world.components;

        let bundle_id = self.world.bundles.init_dynamic_info(
            &mut self.world.storages,
            components,
            component_ids.as_slice(),
        );

        // SAFETY: We just created the bundle, and the archetype is valid, since we are in it.
        let Some(mut remover) = (unsafe {
            BundleRemover::new_with_id(self.world, location.archetype_id, bundle_id, false)
        }) else {
            return self;
        };
        // SAFETY: The remover archetype came from the passed location and the removal can not fail.
        let new_location = unsafe {
            remover.remove(
                self.entity,
                location,
                caller,
                BundleRemover::empty_pre_remove,
            )
        }
        .0;

        self.location = Some(new_location);
        self.world.flush();
        self.update_location();
        self
    }

    /// Despawns the entity without freeing it to the allocator.
    /// This returns the new [`Entity`], which you must manage.
    /// Note that this still increases the generation to differentiate different spawns of the same row.
    ///
    /// This may be later [`spawn_at`](World::spawn_at).
    /// See [`World::despawn_no_free`] for details and usage examples.
    #[track_caller]
    pub fn despawn_no_free(mut self) -> Entity {
        self.despawn_no_free_with_caller(MaybeLocation::caller());
        self.entity
    }

    /// This despawns this entity if it is currently spawned, storing the new [`EntityGeneration`](crate::entity::EntityGeneration) in [`Self::entity`] but not freeing it.
    pub(crate) fn despawn_no_free_with_caller(&mut self, caller: MaybeLocation) {
        // setup
        let Some(location) = self.location else {
            // If there is no location, we are already despawned
            return;
        };
        let archetype = &self.world.archetypes[location.archetype_id];

        // SAFETY: Archetype cannot be mutably aliased by DeferredWorld
        let (archetype, mut deferred_world) = unsafe {
            let archetype: *const Archetype = archetype;
            let world = self.world.as_unsafe_world_cell();
            (&*archetype, world.into_deferred())
        };

        // SAFETY: All components in the archetype exist in world
        unsafe {
            if archetype.has_despawn_observer() {
                // SAFETY: the DESPAWN event_key corresponds to the Despawn event's type
                deferred_world.trigger_raw(
                    DESPAWN,
                    &mut Despawn {
                        entity: self.entity,
                    },
                    &mut EntityComponentsTrigger {
                        components: archetype.components(),
                    },
                    caller,
                );
            }
            deferred_world.trigger_on_despawn(
                archetype,
                self.entity,
                archetype.iter_components(),
                caller,
            );
            if archetype.has_replace_observer() {
                // SAFETY: the REPLACE event_key corresponds to the Replace event's type
                deferred_world.trigger_raw(
                    REPLACE,
                    &mut Replace {
                        entity: self.entity,
                    },
                    &mut EntityComponentsTrigger {
                        components: archetype.components(),
                    },
                    caller,
                );
            }
            deferred_world.trigger_on_replace(
                archetype,
                self.entity,
                archetype.iter_components(),
                caller,
                RelationshipHookMode::Run,
            );
            if archetype.has_remove_observer() {
                // SAFETY: the REMOVE event_key corresponds to the Remove event's type
                deferred_world.trigger_raw(
                    REMOVE,
                    &mut Remove {
                        entity: self.entity,
                    },
                    &mut EntityComponentsTrigger {
                        components: archetype.components(),
                    },
                    caller,
                );
            }
            deferred_world.trigger_on_remove(
                archetype,
                self.entity,
                archetype.iter_components(),
                caller,
            );
        }

        // do the despawn
        let change_tick = self.world.change_tick();
        for component_id in archetype.components() {
            self.world
                .removed_components
                .write(*component_id, self.entity);
        }
        // SAFETY: Since we had a location, and it was valid, this is safe.
        unsafe {
            let was_at = self
                .world
                .entities
                .update_existing_location(self.entity.index(), None);
            debug_assert_eq!(was_at, Some(location));
            self.world
                .entities
                .mark_spawned_or_despawned(self.entity.index(), caller, change_tick);
        }

        let table_row;
        let moved_entity;
        {
            let archetype = &mut self.world.archetypes[location.archetype_id];
            let remove_result = archetype.swap_remove(location.archetype_row);
            if let Some(swapped_entity) = remove_result.swapped_entity {
                let swapped_location = self.world.entities.get_spawned(swapped_entity).unwrap();
                // SAFETY: swapped_entity is valid and the swapped entity's components are
                // moved to the new location immediately after.
                unsafe {
                    self.world.entities.update_existing_location(
                        swapped_entity.index(),
                        Some(EntityLocation {
                            archetype_id: swapped_location.archetype_id,
                            archetype_row: location.archetype_row,
                            table_id: swapped_location.table_id,
                            table_row: swapped_location.table_row,
                        }),
                    );
                }
            }
            table_row = remove_result.table_row;

            for component_id in archetype.sparse_set_components() {
                // set must have existed for the component to be added.
                let sparse_set = self
                    .world
                    .storages
                    .sparse_sets
                    .get_mut(component_id)
                    .unwrap();
                sparse_set.remove(self.entity);
            }
            // SAFETY: table rows stored in archetypes always exist
            moved_entity = unsafe {
                self.world.storages.tables[archetype.table_id()].swap_remove_unchecked(table_row)
            };
        };

        // Handle displaced entity
        if let Some(moved_entity) = moved_entity {
            let moved_location = self.world.entities.get_spawned(moved_entity).unwrap();
            // SAFETY: `moved_entity` is valid and the provided `EntityLocation` accurately reflects
            //         the current location of the entity and its component data.
            unsafe {
                self.world.entities.update_existing_location(
                    moved_entity.index(),
                    Some(EntityLocation {
                        archetype_id: moved_location.archetype_id,
                        archetype_row: moved_location.archetype_row,
                        table_id: moved_location.table_id,
                        table_row,
                    }),
                );
            }
            self.world.archetypes[moved_location.archetype_id]
                .set_entity_table_row(moved_location.archetype_row, table_row);
        }

        // finish
        // SAFETY: We just despawned it.
        self.entity = unsafe { self.world.entities.mark_free(self.entity.index(), 1) };
        self.world.flush();
    }

    /// Despawns the current entity.
    ///
    /// See [`World::despawn`] for more details.
    ///
    /// # Note
    ///
    /// This will also despawn any [`Children`](crate::hierarchy::Children) entities, and any other [`RelationshipTarget`](crate::relationship::RelationshipTarget) that is configured
    /// to despawn descendants. This results in "recursive despawn" behavior.
    #[track_caller]
    pub fn despawn(self) {
        self.despawn_with_caller(MaybeLocation::caller());
    }

    pub(crate) fn despawn_with_caller(mut self, caller: MaybeLocation) {
        self.despawn_no_free_with_caller(caller);
        if let Ok(None) = self.world.entities.get(self.entity) {
            self.world.allocator.free(self.entity);
        }

        // Otherwise:
        // A command must have reconstructed it (had a location); don't free
        // A command must have already despawned it (err) or otherwise made the free unneeded (ex by spawning and despawning in commands); don't free
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
        self.location = self.world.entities().get(self.entity)
            .expect("Attempted to update the location of a despawned entity, which is impossible. This was the result of performing an operation on this EntityWorldMut that queued a despawn command");
    }

    /// Returns if the entity has been despawned.
    ///
    /// Normally it shouldn't be needed to explicitly check if the entity has been despawned
    /// between commands as this shouldn't happen. However, for some special cases where it
    /// is known that a hook or an observer might despawn the entity while a [`EntityWorldMut`]
    /// reference is still held, this method can be used to check if the entity is still alive
    /// to avoid panicking when calling further methods.
    #[inline]
    pub fn is_despawned(&self) -> bool {
        self.location.is_none()
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
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 4);
    ///
    /// # let mut entity = world.get_entity_mut(entity_id).unwrap();
    /// entity.entry::<Comp>().and_modify(|mut c| c.0 += 1);
    /// assert_eq!(world.query::<&Comp>().single(&world).unwrap().0, 5);
    /// ```
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    pub fn entry<'a, T: Component>(&'a mut self) -> ComponentEntry<'w, 'a, T> {
        if self.contains::<T>() {
            ComponentEntry::Occupied(OccupiedComponentEntry {
                entity_world: self,
                _marker: PhantomData,
            })
        } else {
            ComponentEntry::Vacant(VacantComponentEntry {
                entity_world: self,
                _marker: PhantomData,
            })
        }
    }

    /// Creates an [`Observer`] watching for an [`TargetEvent`] of type `E` that targets this entity.
    ///
    /// # Panics
    ///
    /// If the entity has been despawned while this `EntityWorldMut` is still alive.
    ///
    /// Panics if the given system is an exclusive system.
    #[track_caller]
    pub fn observe<E: TargetEvent, B: Bundle, M>(
        &mut self,
        observer: impl IntoObserverSystem<E, B, M>,
    ) -> &mut Self {
        self.observe_with_caller(observer, MaybeLocation::caller())
    }

    pub(crate) fn observe_with_caller<E: TargetEvent, B: Bundle, M>(
        &mut self,
        observer: impl IntoObserverSystem<E, B, M>,
        caller: MaybeLocation,
    ) -> &mut Self {
        self.assert_not_despawned();
        let bundle = Observer::new(observer).with_entity(self.entity);
        move_as_ptr!(bundle);
        self.world.spawn_with_caller(bundle, caller);
        self.world.flush();
        self.update_location();
        self
    }

    /// Clones parts of an entity (components, observers, etc.) onto another entity,
    /// configured through [`EntityClonerBuilder`].
    ///
    /// The other entity will receive all the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect) except those that are
    /// [denied](EntityClonerBuilder::deny) in the `config`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Component, Clone, PartialEq, Debug)]
    /// # struct ComponentA;
    /// # #[derive(Component, Clone, PartialEq, Debug)]
    /// # struct ComponentB;
    /// # let mut world = World::new();
    /// # let entity = world.spawn((ComponentA, ComponentB)).id();
    /// # let target = world.spawn_empty().id();
    /// // Clone all components except ComponentA onto the target.
    /// world.entity_mut(entity).clone_with_opt_out(target, |builder| {
    ///     builder.deny::<ComponentA>();
    /// });
    /// # assert_eq!(world.get::<ComponentA>(target), None);
    /// # assert_eq!(world.get::<ComponentB>(target), Some(&ComponentB));
    /// ```
    ///
    /// See [`EntityClonerBuilder<OptOut>`] for more options.
    ///
    /// # Panics
    ///
    /// - If this entity has been despawned while this `EntityWorldMut` is still alive.
    /// - If the target entity does not exist.
    pub fn clone_with_opt_out(
        &mut self,
        target: Entity,
        config: impl FnOnce(&mut EntityClonerBuilder<OptOut>) + Send + Sync + 'static,
    ) -> &mut Self {
        self.assert_not_despawned();

        let mut builder = EntityCloner::build_opt_out(self.world);
        config(&mut builder);
        builder.clone_entity(self.entity, target);

        self.world.flush();
        self.update_location();
        self
    }

    /// Clones parts of an entity (components, observers, etc.) onto another entity,
    /// configured through [`EntityClonerBuilder`].
    ///
    /// The other entity will receive only the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect) and are
    /// [allowed](EntityClonerBuilder::allow) in the `config`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # #[derive(Component, Clone, PartialEq, Debug)]
    /// # struct ComponentA;
    /// # #[derive(Component, Clone, PartialEq, Debug)]
    /// # struct ComponentB;
    /// # let mut world = World::new();
    /// # let entity = world.spawn((ComponentA, ComponentB)).id();
    /// # let target = world.spawn_empty().id();
    /// // Clone only ComponentA onto the target.
    /// world.entity_mut(entity).clone_with_opt_in(target, |builder| {
    ///     builder.allow::<ComponentA>();
    /// });
    /// # assert_eq!(world.get::<ComponentA>(target), Some(&ComponentA));
    /// # assert_eq!(world.get::<ComponentB>(target), None);
    /// ```
    ///
    /// See [`EntityClonerBuilder<OptIn>`] for more options.
    ///
    /// # Panics
    ///
    /// - If this entity has been despawned while this `EntityWorldMut` is still alive.
    /// - If the target entity does not exist.
    pub fn clone_with_opt_in(
        &mut self,
        target: Entity,
        config: impl FnOnce(&mut EntityClonerBuilder<OptIn>) + Send + Sync + 'static,
    ) -> &mut Self {
        self.assert_not_despawned();

        let mut builder = EntityCloner::build_opt_in(self.world);
        config(&mut builder);
        builder.clone_entity(self.entity, target);

        self.world.flush();
        self.update_location();
        self
    }

    /// Spawns a clone of this entity and returns the [`Entity`] of the clone.
    ///
    /// The clone will receive all the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect).
    ///
    /// To configure cloning behavior (such as only cloning certain components),
    /// use [`EntityWorldMut::clone_and_spawn_with_opt_out`]/
    /// [`opt_in`](`EntityWorldMut::clone_and_spawn_with_opt_in`).
    ///
    /// # Panics
    ///
    /// If this entity has been despawned while this `EntityWorldMut` is still alive.
    pub fn clone_and_spawn(&mut self) -> Entity {
        self.clone_and_spawn_with_opt_out(|_| {})
    }

    /// Spawns a clone of this entity and allows configuring cloning behavior
    /// using [`EntityClonerBuilder`], returning the [`Entity`] of the clone.
    ///
    /// The clone will receive all the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect) except those that are
    /// [denied](EntityClonerBuilder::deny) in the `config`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let entity = world.spawn((ComponentA, ComponentB)).id();
    /// # #[derive(Component, Clone, PartialEq, Debug)]
    /// # struct ComponentA;
    /// # #[derive(Component, Clone, PartialEq, Debug)]
    /// # struct ComponentB;
    /// // Create a clone of an entity but without ComponentA.
    /// let entity_clone = world.entity_mut(entity).clone_and_spawn_with_opt_out(|builder| {
    ///     builder.deny::<ComponentA>();
    /// });
    /// # assert_eq!(world.get::<ComponentA>(entity_clone), None);
    /// # assert_eq!(world.get::<ComponentB>(entity_clone), Some(&ComponentB));
    /// ```
    ///
    /// See [`EntityClonerBuilder<OptOut>`] for more options.
    ///
    /// # Panics
    ///
    /// If this entity has been despawned while this `EntityWorldMut` is still alive.
    pub fn clone_and_spawn_with_opt_out(
        &mut self,
        config: impl FnOnce(&mut EntityClonerBuilder<OptOut>) + Send + Sync + 'static,
    ) -> Entity {
        self.assert_not_despawned();
        let entity_clone = self.world.spawn_empty().id();

        let mut builder = EntityCloner::build_opt_out(self.world);
        config(&mut builder);
        builder.clone_entity(self.entity, entity_clone);

        self.world.flush();
        self.update_location();
        entity_clone
    }

    /// Spawns a clone of this entity and allows configuring cloning behavior
    /// using [`EntityClonerBuilder`], returning the [`Entity`] of the clone.
    ///
    /// The clone will receive only the components of the original that implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect) and are
    /// [allowed](EntityClonerBuilder::allow) in the `config`.
    ///
    /// # Example
    ///
    /// ```
    /// # use bevy_ecs::prelude::*;
    /// # let mut world = World::new();
    /// # let entity = world.spawn((ComponentA, ComponentB)).id();
    /// # #[derive(Component, Clone, PartialEq, Debug)]
    /// # struct ComponentA;
    /// # #[derive(Component, Clone, PartialEq, Debug)]
    /// # struct ComponentB;
    /// // Create a clone of an entity but only with ComponentA.
    /// let entity_clone = world.entity_mut(entity).clone_and_spawn_with_opt_in(|builder| {
    ///     builder.allow::<ComponentA>();
    /// });
    /// # assert_eq!(world.get::<ComponentA>(entity_clone), Some(&ComponentA));
    /// # assert_eq!(world.get::<ComponentB>(entity_clone), None);
    /// ```
    ///
    /// See [`EntityClonerBuilder<OptIn>`] for more options.
    ///
    /// # Panics
    ///
    /// If this entity has been despawned while this `EntityWorldMut` is still alive.
    pub fn clone_and_spawn_with_opt_in(
        &mut self,
        config: impl FnOnce(&mut EntityClonerBuilder<OptIn>) + Send + Sync + 'static,
    ) -> Entity {
        self.assert_not_despawned();
        let entity_clone = self.world.spawn_empty().id();

        let mut builder = EntityCloner::build_opt_in(self.world);
        config(&mut builder);
        builder.clone_entity(self.entity, entity_clone);

        self.world.flush();
        self.update_location();
        entity_clone
    }

    /// Clones the specified components of this entity and inserts them into another entity.
    ///
    /// Components can only be cloned if they implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect).
    ///
    /// # Panics
    ///
    /// - If this entity has been despawned while this `EntityWorldMut` is still alive.
    /// - If the target entity does not exist.
    pub fn clone_components<B: Bundle>(&mut self, target: Entity) -> &mut Self {
        self.assert_not_despawned();

        EntityCloner::build_opt_in(self.world)
            .allow::<B>()
            .clone_entity(self.entity, target);

        self.world.flush();
        self.update_location();
        self
    }

    /// Clones the specified components of this entity and inserts them into another entity,
    /// then removes the components from this entity.
    ///
    /// Components can only be cloned if they implement
    /// [`Clone`] or [`Reflect`](bevy_reflect::Reflect).
    ///
    /// # Panics
    ///
    /// - If this entity has been despawned while this `EntityWorldMut` is still alive.
    /// - If the target entity does not exist.
    pub fn move_components<B: Bundle>(&mut self, target: Entity) -> &mut Self {
        self.assert_not_despawned();

        EntityCloner::build_opt_in(self.world)
            .allow::<B>()
            .move_components(true)
            .clone_entity(self.entity, target);

        self.world.flush();
        self.update_location();
        self
    }

    /// Returns the source code location from which this entity has last been spawned.
    pub fn spawned_by(&self) -> MaybeLocation {
        self.world()
            .entities()
            .entity_get_spawned_or_despawned_by(self.entity)
            .map(|location| location.unwrap())
    }

    /// Returns the [`Tick`] at which this entity has last been spawned.
    pub fn spawn_tick(&self) -> Tick {
        self.assert_not_despawned();

        // SAFETY: entity being alive was asserted
        unsafe {
            self.world()
                .entities()
                .entity_get_spawned_or_despawned_unchecked(self.entity)
                .1
        }
    }

    /// Reborrows this entity in a temporary scope.
    /// This is useful for executing a function that requires a `EntityWorldMut`
    /// but you do not want to move out the entity ownership.
    pub fn reborrow_scope<U>(&mut self, f: impl FnOnce(EntityWorldMut) -> U) -> U {
        let Self {
            entity, location, ..
        } = *self;
        self.world_scope(move |world| {
            f(EntityWorldMut {
                world,
                entity,
                location,
            })
        })
    }

    /// Passes the current entity into the given function, and triggers the event returned by that function.
    /// See [`IntoTargetEvent`] for usage examples.
    #[track_caller]
    pub fn trigger<M, T: IntoTargetEvent<M>>(&mut self, event_fn: T) -> &mut Self {
        let (mut event, mut trigger) = event_fn.into_event_from_entity(self.entity);
        let caller = MaybeLocation::caller();
        self.world_scope(|world| {
            world.trigger_ref_with_caller(&mut event, &mut trigger, caller);
        });
        self
    }
}

impl<'w> From<EntityWorldMut<'w>> for EntityRef<'w> {
    fn from(entity: EntityWorldMut<'w>) -> EntityRef<'w> {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe { EntityRef::new(entity.into_unsafe_entity_cell()) }
    }
}

impl<'a> From<&'a EntityWorldMut<'_>> for EntityRef<'a> {
    fn from(entity: &'a EntityWorldMut<'_>) -> Self {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        // - `&entity` ensures no mutable accesses are active.
        unsafe { EntityRef::new(entity.as_unsafe_entity_cell_readonly()) }
    }
}

impl<'w> From<EntityWorldMut<'w>> for EntityMut<'w> {
    fn from(entity: EntityWorldMut<'w>) -> Self {
        // SAFETY: `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe { EntityMut::new(entity.into_unsafe_entity_cell()) }
    }
}

impl<'a> From<&'a mut EntityWorldMut<'_>> for EntityMut<'a> {
    #[inline]
    fn from(entity: &'a mut EntityWorldMut<'_>) -> Self {
        // SAFETY: `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe { EntityMut::new(entity.as_unsafe_entity_cell()) }
    }
}

impl<'a> From<EntityWorldMut<'a>> for FilteredEntityRef<'a, 'static> {
    fn from(entity: EntityWorldMut<'a>) -> Self {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe {
            FilteredEntityRef::new(
                entity.into_unsafe_entity_cell(),
                const { &Access::new_read_all() },
            )
        }
    }
}

impl<'a> From<&'a EntityWorldMut<'_>> for FilteredEntityRef<'a, 'static> {
    fn from(entity: &'a EntityWorldMut<'_>) -> Self {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe {
            FilteredEntityRef::new(
                entity.as_unsafe_entity_cell_readonly(),
                const { &Access::new_read_all() },
            )
        }
    }
}

impl<'a> From<EntityWorldMut<'a>> for FilteredEntityMut<'a, 'static> {
    fn from(entity: EntityWorldMut<'a>) -> Self {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe {
            FilteredEntityMut::new(
                entity.into_unsafe_entity_cell(),
                const { &Access::new_write_all() },
            )
        }
    }
}

impl<'a> From<&'a mut EntityWorldMut<'_>> for FilteredEntityMut<'a, 'static> {
    fn from(entity: &'a mut EntityWorldMut<'_>) -> Self {
        // SAFETY:
        // - `EntityWorldMut` guarantees exclusive access to the entire world.
        unsafe {
            FilteredEntityMut::new(
                entity.as_unsafe_entity_cell(),
                const { &Access::new_write_all() },
            )
        }
    }
}

/// Inserts a dynamic [`Bundle`] into the entity.
///
/// # Safety
///
/// - [`OwningPtr`] and [`StorageType`] iterators must correspond to the
///   [`BundleInfo`](crate::bundle::BundleInfo) used to construct [`BundleInserter`]
/// - [`Entity`] must correspond to [`EntityLocation`]
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
    mode: InsertMode,
    caller: MaybeLocation,
    relationship_hook_insert_mode: RelationshipHookMode,
) -> EntityLocation {
    struct DynamicInsertBundle<'a, I: Iterator<Item = (StorageType, OwningPtr<'a>)>> {
        components: I,
    }

    impl<'a, I: Iterator<Item = (StorageType, OwningPtr<'a>)>> DynamicBundle
        for DynamicInsertBundle<'a, I>
    {
        type Effect = ();
        unsafe fn get_components(
            mut ptr: MovingPtr<'_, Self>,
            func: &mut impl FnMut(StorageType, OwningPtr<'_>),
        ) {
            (&mut ptr.components).for_each(|(t, ptr)| func(t, ptr));
        }

        unsafe fn apply_effect(
            _ptr: MovingPtr<'_, MaybeUninit<Self>>,
            _entity: &mut EntityWorldMut,
        ) {
        }
    }

    let bundle = DynamicInsertBundle {
        components: storage_types.zip(components),
    };

    move_as_ptr!(bundle);

    // SAFETY:
    // - `location` matches `entity`.  and thus must currently exist in the source
    //   archetype for this inserter and its location within the archetype.
    // - The caller must ensure that the iterators and storage types match up with the `BundleInserter`
    // - `apply_effect` is never called on this bundle.
    // - `bundle` is not used or dropped after this point.
    unsafe {
        bundle_inserter.insert(
            entity,
            location,
            bundle,
            mode,
            caller,
            relationship_hook_insert_mode,
        )
    }
}
