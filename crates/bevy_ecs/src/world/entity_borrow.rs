use std::any::TypeId;

use bevy_ptr::Ptr;

use crate::{
    archetype::Archetype,
    change_detection::{MutUntyped, Ref},
    component::{ComponentId, ComponentTicks},
    entity::{Entity, EntityLocation},
    prelude::Component,
    world::unsafe_world_cell::UnsafeEntityCell,
};

use super::{EntityMut, EntityRef, Mut};

/// Provides read-only access to a single entity and all of its components.
///
/// Contrast with [`EntityRef`], which provides access to the entire world
/// in addition to an entity. Because of this, `EntityRef` can not be used
/// in the same system as any mutable access (unless a [`ParamSet`] is used).
///
/// [`ParamSet`]: crate::system::ParamSet
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
///     query1: Query<EntityBorrow, With<A>>,
///     query2: Query<&mut B, Without<A>>,
/// ) {
///     // ...
/// }
/// # bevy_ecs::system::assert_is_system(disjoint_system);
/// ```
#[derive(Clone, Copy)]
pub struct EntityBorrow<'w>(UnsafeEntityCell<'w>);

impl<'w> From<EntityRef<'w>> for EntityBorrow<'w> {
    fn from(value: EntityRef<'w>) -> Self {
        // SAFETY: `EntityRef` guarantees shared access to the entire world.
        unsafe { EntityBorrow::new(value.as_unsafe_entity_cell()) }
    }
}

impl<'a> From<&'a EntityMut<'_>> for EntityBorrow<'a> {
    fn from(value: &'a EntityMut<'_>) -> Self {
        // SAFETY:
        // - `EntityMut` guarantees exclusive access to the entire world.
        // - `&self` ensures no mutable accesses are active.
        unsafe { EntityBorrow::new(value.as_unsafe_entity_cell_readonly()) }
    }
}

impl<'w> From<EntityBorrowMut<'w>> for EntityBorrow<'w> {
    fn from(value: EntityBorrowMut<'w>) -> Self {
        // SAFETY:
        // - `EntityBorrowMut` gurantees exclusive access to the world.
        unsafe { EntityBorrow::new(value.0) }
    }
}

impl<'w> EntityBorrow<'w> {
    /// # Safety
    /// - `cell` must have permission to read every component of the entity.
    /// - No mutable accesses to any of the entity's components may exist
    ///   at the same time as the returned [`EntityBorrow`].
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
    /// [`Self::contains_type_id`].
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

    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`EntityRef::get`], this returns a raw pointer to the component,
    /// which is only valid while the `'w` borrow of the lifetime is active.
    #[inline]
    pub fn get_by_id(&self, component_id: ComponentId) -> Option<Ptr<'w>> {
        // SAFETY: We have read-only access to all components of this entity.
        unsafe { self.0.get_by_id(component_id) }
    }
}

/// Provides mutable access to a single entity and all of its components.
///
/// Contrast with [`EntityMut`], with allows adding adn removing components,
/// despawning the entity, and provides mutable access to the eentire world.
/// Because of this, `EntityMut` cannot coexist with any other world accesses.
///
/// [`EntityMut`]: super::EntityMut
///
/// # Examples
///
/// Disjoint mutable access.
///
/// ```
/// # use bevy_ecs::prelude::*;
/// # #[derive(Component)] pub struct A;
/// fn disjoint_system(
///     query1: Query<EntityBorrow, With<A>>,
///     query2: Query<EntityBorrow, Without<A>>,
/// ) {
///     // ...
/// }
/// # bevy_ecs::system::assert_is_system(disjoint_system);
/// ```
pub struct EntityBorrowMut<'w>(UnsafeEntityCell<'w>);

impl<'w> From<EntityMut<'w>> for EntityBorrowMut<'w> {
    fn from(value: EntityMut<'w>) -> Self {
        // SAFETY: `EntityMut` guarantees exclusive access to the entire world.
        unsafe { EntityBorrowMut::new(value.into_unsafe_entity_cell()) }
    }
}

impl<'a> From<&'a mut EntityMut<'_>> for EntityBorrowMut<'a> {
    fn from(value: &'a mut EntityMut<'_>) -> Self {
        // SAFETY: `EntityMut` guarantees exclusive access to the entire world.
        unsafe { EntityBorrowMut::new(value.as_unsafe_entity_cell()) }
    }
}

impl<'w> EntityBorrowMut<'w> {
    /// # Safety
    /// - `cell` must have permission to mutate every component of the entity.
    /// - No accesses to any of the entity's components may exist
    ///   at the same time as the returned [`EntityBorrowMut`].
    pub(crate) unsafe fn new(cell: UnsafeEntityCell<'w>) -> Self {
        Self(cell)
    }

    /// Returns a new instance with a shorter lifetime.
    /// This is useful if you have `&mut EntityBorrowMut`, but you need `EntityBorrowMut`.
    pub fn reborrow(&mut self) -> EntityBorrowMut<'_> {
        // SAFETY: We have exclusive access to the entire entity and its components.
        unsafe { Self::new(self.0) }
    }

    /// Gets read-only access to all of the entity's components.
    pub fn as_readonly(&self) -> EntityBorrow<'_> {
        // SAFETY:
        // - This type gurantees exclusive access to the world.
        // - `&self` ensures there are no mutable accesses.
        unsafe { EntityBorrow::new(self.0) }
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
    /// [`Self::contains_type_id`].
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
        // SAFETY: &mut self implies exclusive access for duration of returned value
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
    /// **You should prefer to use the typed API [`EntityMut::get_change_ticks`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    #[inline]
    pub fn get_change_ticks_by_id(&self, component_id: ComponentId) -> Option<ComponentTicks> {
        self.as_readonly().get_change_ticks_by_id(component_id)
    }

    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`EntityMut::get`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`EntityBorrowMut::get`], this returns a raw pointer to the component,
    /// which is only valid while the [`EntityBorrowMut`] is alive.
    #[inline]
    pub fn get_by_id(&self, component_id: ComponentId) -> Option<Ptr<'_>> {
        self.as_readonly().get_by_id(component_id)
    }

    /// Gets a [`MutUntyped`] of the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API [`EntityMut::get_mut`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`EntityMut::get_mut`], this returns a raw pointer to the component,
    /// which is only valid while the [`EntityMut`] is alive.
    #[inline]
    pub fn get_mut_by_id(&mut self, component_id: ComponentId) -> Option<MutUntyped<'_>> {
        // SAFETY:
        // - `&mut self` ensures that no references exist to this entity's components.
        // - `as_unsafe_world_cell` gives mutable permission for all components on this entity
        unsafe { self.0.get_mut_by_id(component_id) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{self as bevy_ecs, prelude::*, system::assert_is_system};

    #[derive(Component)]
    struct A;

    #[test]
    fn disjoint_access() {
        fn disjoint_readonly(
            _: Query<EntityBorrow, With<A>>,
            _: Query<EntityBorrowMut, Without<A>>,
        ) {
        }

        fn disjoint_mutable(
            _: Query<EntityBorrowMut, With<A>>,
            _: Query<EntityBorrowMut, Without<A>>,
        ) {
        }

        assert_is_system(disjoint_readonly);
        assert_is_system(disjoint_mutable);
    }

    #[test]
    fn borrow_compatible() {
        fn borrow_system(_: Query<(EntityBorrow, &A)>, _: Query<&A>) {}

        assert_is_system(borrow_system);
    }

    #[test]
    fn borrow_mut_compatible() {
        fn borrow_mut_system(_: Query<(Entity, EntityBorrowMut)>) {}

        assert_is_system(borrow_mut_system);
    }

    #[test]
    #[should_panic]
    fn borrow_mut_incompatible1() {
        fn incompatible_system(_: Query<(EntityBorrowMut, &A)>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    #[should_panic]
    fn borrow_mut_incompatible2() {
        fn incompatible_system(_: Query<(EntityBorrowMut, &mut A)>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    #[should_panic]
    fn borrow_mut_incompatible3() {
        fn incompatible_system(_: Query<EntityBorrowMut>, _: Query<&A>) {}

        assert_is_system(incompatible_system);
    }

    #[test]
    #[should_panic]
    fn borrow_mut_incompatible4() {
        fn incompatible_system(_: Query<EntityBorrowMut>, _: Query<&mut A>) {}

        assert_is_system(incompatible_system);
    }
}
