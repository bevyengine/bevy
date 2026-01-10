use crate::{
    bundle::Bundle,
    change_detection::{ComponentTicks, MaybeLocation, MutUntyped, Tick},
    component::{Component, ComponentId, Components, Mutable},
    entity::{ContainsEntity, Entity, EntityEquivalent},
    query::Access,
    world::{
        unsafe_world_cell::UnsafeEntityCell, DynamicComponentFetch, FilteredEntityMut,
        FilteredEntityRef, Mut, Ref,
    },
};

use bevy_ptr::Ptr;
use core::{
    any::TypeId,
    cmp::Ordering,
    hash::{Hash, Hasher},
    marker::PhantomData,
};

/// Provides read-only access to a single entity and all its components, save
/// for an explicitly-enumerated set.
pub struct EntityRefExcept<'w, 's, B>
where
    B: Bundle,
{
    entity: UnsafeEntityCell<'w>,
    access: &'s Access,
    phantom: PhantomData<B>,
}

impl<'w, 's, B> EntityRefExcept<'w, 's, B>
where
    B: Bundle,
{
    /// # Safety
    /// Other users of `UnsafeEntityCell` must only have mutable access to the components in `B`.
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>, access: &'s Access) -> Self {
        Self {
            entity,
            access,
            phantom: PhantomData,
        }
    }

    /// Consumes `self` and returns a [`FilteredEntityRef`], which provides
    /// read-only access to all of the entity's components, except for the ones
    /// in `B`.
    #[inline]
    pub fn into_filtered(self) -> FilteredEntityRef<'w, 's> {
        // SAFETY:
        // - The FilteredEntityRef has the same component access as the given EntityRefExcept.
        unsafe { FilteredEntityRef::new(self.entity, self.access) }
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
        let id = components.valid_component_id::<C>()?;
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
        let id = components.valid_component_id::<C>()?;
        if bundle_contains_component::<B>(components, id) {
            None
        } else {
            // SAFETY: We have read access for all components that weren't
            // covered by the `contains` check above.
            unsafe { self.entity.get_ref() }
        }
    }

    /// Returns the source code location from which this entity has been spawned.
    pub fn spawned_by(&self) -> MaybeLocation {
        self.entity.spawned_by()
    }

    /// Returns the [`Tick`] at which this entity has been spawned.
    pub fn spawn_tick(&self) -> Tick {
        self.entity.spawn_tick()
    }

    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`Self::get`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`EntityRefExcept::get`], this returns a raw pointer to the component,
    /// which is only valid while the [`EntityRefExcept`] is alive.
    #[inline]
    pub fn get_by_id(&self, component_id: ComponentId) -> Option<Ptr<'w>> {
        let components = self.entity.world().components();
        (!bundle_contains_component::<B>(components, component_id))
            .then(|| {
                // SAFETY: We have read access for this component
                unsafe { self.entity.get_by_id(component_id) }
            })
            .flatten()
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

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    #[inline]
    pub fn get_change_ticks<T: Component>(&self) -> Option<ComponentTicks> {
        let component_id = self
            .entity
            .world()
            .components()
            .get_valid_id(TypeId::of::<T>())?;
        let components = self.entity.world().components();
        (!bundle_contains_component::<B>(components, component_id))
            .then(|| {
                // SAFETY: We have read access
                unsafe { self.entity.get_change_ticks::<T>() }
            })
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
        let components = self.entity.world().components();
        (!bundle_contains_component::<B>(components, component_id))
            .then(|| {
                // SAFETY: We have read access
                unsafe { self.entity.get_change_ticks_by_id(component_id) }
            })
            .flatten()
    }
}

impl<'w, 's, B: Bundle> From<EntityRefExcept<'w, 's, B>> for FilteredEntityRef<'w, 's> {
    fn from(entity: EntityRefExcept<'w, 's, B>) -> Self {
        entity.into_filtered()
    }
}

impl<'w, 's, B: Bundle> From<&'w EntityRefExcept<'_, 's, B>> for FilteredEntityRef<'w, 's> {
    fn from(entity: &'w EntityRefExcept<'_, 's, B>) -> Self {
        entity.into_filtered()
    }
}

impl<B: Bundle> Clone for EntityRefExcept<'_, '_, B> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<B: Bundle> Copy for EntityRefExcept<'_, '_, B> {}

impl<B: Bundle> PartialEq for EntityRefExcept<'_, '_, B> {
    fn eq(&self, other: &Self) -> bool {
        self.entity() == other.entity()
    }
}

impl<B: Bundle> Eq for EntityRefExcept<'_, '_, B> {}

impl<B: Bundle> PartialOrd for EntityRefExcept<'_, '_, B> {
    /// [`EntityRefExcept`]'s comparison trait implementations match the underlying [`Entity`],
    /// and cannot discern between different worlds.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<B: Bundle> Ord for EntityRefExcept<'_, '_, B> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.entity().cmp(&other.entity())
    }
}

impl<B: Bundle> Hash for EntityRefExcept<'_, '_, B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity().hash(state);
    }
}

impl<B: Bundle> ContainsEntity for EntityRefExcept<'_, '_, B> {
    fn entity(&self) -> Entity {
        self.id()
    }
}

// SAFETY: This type represents one Entity. We implement the comparison traits based on that Entity.
unsafe impl<B: Bundle> EntityEquivalent for EntityRefExcept<'_, '_, B> {}

/// Provides mutable access to all components of an entity, with the exception
/// of an explicit set.
///
/// This is a rather niche type that should only be used if you need access to
/// *all* components of an entity, while still allowing you to consult other
/// queries that might match entities that this query also matches. If you don't
/// need access to all components, prefer a standard query with a
/// [`Without`](`crate::query::Without`) filter.
pub struct EntityMutExcept<'w, 's, B>
where
    B: Bundle,
{
    entity: UnsafeEntityCell<'w>,
    access: &'s Access,
    phantom: PhantomData<B>,
}

impl<'w, 's, B> EntityMutExcept<'w, 's, B>
where
    B: Bundle,
{
    /// # Safety
    /// Other users of `UnsafeEntityCell` must not have access to any components not in `B`.
    pub(crate) unsafe fn new(entity: UnsafeEntityCell<'w>, access: &'s Access) -> Self {
        Self {
            entity,
            access,
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
    #[inline]
    pub fn reborrow(&mut self) -> EntityMutExcept<'_, 's, B> {
        // SAFETY: We have exclusive access to the entire entity and the
        // applicable components.
        unsafe { Self::new(self.entity, self.access) }
    }

    /// Consumes `self` and returns read-only access to all of the entity's
    /// components, except for the ones in `B`.
    #[inline]
    pub fn into_readonly(self) -> EntityRefExcept<'w, 's, B> {
        // SAFETY: All accesses that `EntityRefExcept` provides are also
        // accesses that `EntityMutExcept` provides.
        unsafe { EntityRefExcept::new(self.entity, self.access) }
    }

    /// Gets read-only access to all of the entity's components, except for the
    /// ones in `B`.
    #[inline]
    pub fn as_readonly(&self) -> EntityRefExcept<'_, 's, B> {
        // SAFETY: All accesses that `EntityRefExcept` provides are also
        // accesses that `EntityMutExcept` provides.
        unsafe { EntityRefExcept::new(self.entity, self.access) }
    }

    /// Consumes `self` and returns a [`FilteredEntityMut`], which provides
    /// mutable access to all of the entity's components, except for the ones in
    /// `B`.
    #[inline]
    pub fn into_filtered(self) -> FilteredEntityMut<'w, 's> {
        // SAFETY:
        // - The FilteredEntityMut has the same component access as the given EntityMutExcept.
        unsafe { FilteredEntityMut::new(self.entity, self.access) }
    }

    /// Get access to the underlying [`UnsafeEntityCell`]
    #[inline]
    pub fn as_unsafe_entity_cell(&mut self) -> UnsafeEntityCell<'_> {
        self.entity
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
        C: Component<Mutability = Mutable>,
    {
        let components = self.entity.world().components();
        let id = components.valid_component_id::<C>()?;
        if bundle_contains_component::<B>(components, id) {
            None
        } else {
            // SAFETY: We have write access for all components that weren't
            // covered by the `contains` check above.
            unsafe { self.entity.get_mut() }
        }
    }

    /// Returns the source code location from which this entity has been spawned.
    pub fn spawned_by(&self) -> MaybeLocation {
        self.entity.spawned_by()
    }

    /// Returns the [`Tick`] at which this entity has been spawned.
    pub fn spawn_tick(&self) -> Tick {
        self.entity.spawn_tick()
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

    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`Self::get`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`EntityMutExcept::get`], this returns a raw pointer to the component,
    /// which is only valid while the [`EntityMutExcept`] is alive.
    #[inline]
    pub fn get_by_id(&'w self, component_id: ComponentId) -> Option<Ptr<'w>> {
        self.as_readonly().get_by_id(component_id)
    }

    /// Gets a [`MutUntyped`] of the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`Self::get_mut`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`EntityMutExcept::get_mut`], this returns a raw pointer to the component,
    /// which is only valid while the [`EntityMutExcept`] is alive.
    #[inline]
    pub fn get_mut_by_id<F: DynamicComponentFetch>(
        &mut self,
        component_id: ComponentId,
    ) -> Option<MutUntyped<'_>> {
        let components = self.entity.world().components();
        (!bundle_contains_component::<B>(components, component_id))
            .then(|| {
                // SAFETY: We have write access
                unsafe { self.entity.get_mut_by_id(component_id).ok() }
            })
            .flatten()
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

impl<'w, 's, B: Bundle> From<&'w mut EntityMutExcept<'_, 's, B>> for EntityMutExcept<'w, 's, B> {
    #[inline]
    fn from(entity: &'w mut EntityMutExcept<'_, 's, B>) -> Self {
        entity.reborrow()
    }
}

impl<'w, 's, B: Bundle> From<EntityMutExcept<'w, 's, B>> for EntityRefExcept<'w, 's, B> {
    #[inline]
    fn from(entity: EntityMutExcept<'w, 's, B>) -> Self {
        entity.into_readonly()
    }
}

impl<'w, 's, B: Bundle> From<&'w EntityMutExcept<'_, 's, B>> for EntityRefExcept<'w, 's, B> {
    #[inline]
    fn from(entity: &'w EntityMutExcept<'_, 's, B>) -> Self {
        entity.as_readonly()
    }
}

impl<B: Bundle> PartialEq for EntityMutExcept<'_, '_, B> {
    fn eq(&self, other: &Self) -> bool {
        self.entity() == other.entity()
    }
}

impl<B: Bundle> Eq for EntityMutExcept<'_, '_, B> {}

impl<B: Bundle> PartialOrd for EntityMutExcept<'_, '_, B> {
    /// [`EntityMutExcept`]'s comparison trait implementations match the underlying [`Entity`],
    /// and cannot discern between different worlds.
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<B: Bundle> Ord for EntityMutExcept<'_, '_, B> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.entity().cmp(&other.entity())
    }
}

impl<B: Bundle> Hash for EntityMutExcept<'_, '_, B> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.entity().hash(state);
    }
}

impl<B: Bundle> ContainsEntity for EntityMutExcept<'_, '_, B> {
    fn entity(&self) -> Entity {
        self.id()
    }
}

// SAFETY: This type represents one Entity. We implement the comparison traits based on that Entity.
unsafe impl<B: Bundle> EntityEquivalent for EntityMutExcept<'_, '_, B> {}

fn bundle_contains_component<B>(components: &Components, query_id: ComponentId) -> bool
where
    B: Bundle,
{
    let mut found = false;
    for id in B::get_component_ids(components).flatten() {
        found = found || id == query_id;
    }
    found
}
