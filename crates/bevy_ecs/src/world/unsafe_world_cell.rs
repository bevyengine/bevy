#![warn(unsafe_op_in_unsafe_fn)]

use super::{Mut, World};
use crate::{
    archetype::{Archetype, ArchetypeComponentId, Archetypes},
    bundle::Bundles,
    change_detection::{MutUntyped, TicksMut},
    component::{
        ComponentId, ComponentStorage, ComponentTicks, Components, StorageType, TickCells,
    },
    entity::{Entities, Entity, EntityLocation},
    prelude::Component,
    storage::{Column, ComponentSparseSet},
    system::Resource,
};
use bevy_ptr::Ptr;
use std::{any::TypeId, cell::UnsafeCell, marker::PhantomData, sync::atomic::Ordering};

/// Variant of the [`World`] where resource and component accesses take `&self`, and the responsibility to avoid
/// aliasing violations are given to the caller instead of being checked at compile-time by rust's unique XOR shared rule.
///
/// ### Rationale
/// In rust, having a `&mut World` means that there are absolutely no other references to the safe world alive at the same time,
/// without exceptions. Not even unsafe code can change this.
///
/// But there are situations where careful shared mutable access through a type is possible and safe. For this, rust provides the [`UnsafeCell`](std::cell::UnsafeCell)
/// escape hatch, which allows you to get a `*mut T` from a `&UnsafeCell<T>` and around which safe abstractions can be built.
///
/// Access to resources and components can be done uniquely using [`World::resource_mut`] and [`World::entity_mut`], and shared using [`World::resource`] and [`World::entity`].
/// These methods use lifetimes to check at compile time that no aliasing rules are being broken.
///
/// This alone is not enough to implement bevy systems where multiple systems can access *disjoint* parts of the world concurrently. For this, bevy stores all values of
/// resources and components (and [`ComponentTicks`](crate::component::ComponentTicks)) in [`UnsafeCell`](std::cell::UnsafeCell)s, and carefully validates disjoint access patterns using
/// APIs like [`System::component_access`](crate::system::System::component_access).
///
/// A system then can be executed using [`System::run_unsafe`](crate::system::System::run_unsafe) with a `&World` and use methods with interior mutability to access resource values.
/// access resource values.
///
/// ### Example Usage
///
/// [`UnsafeWorldCell`] can be used as a building block for writing APIs that safely allow disjoint access into the world.
/// In the following example, the world is split into a resource access half and a component access half, where each one can
/// safely hand out mutable references.
///
/// ```
/// use bevy_ecs::world::World;
/// use bevy_ecs::change_detection::Mut;
/// use bevy_ecs::system::Resource;
/// use bevy_ecs::world::unsafe_world_cell::UnsafeWorldCell;
///
/// // INVARIANT: existence of this struct means that users of it are the only ones being able to access resources in the world
/// struct OnlyResourceAccessWorld<'w>(UnsafeWorldCell<'w>);
/// // INVARIANT: existence of this struct means that users of it are the only ones being able to access components in the world
/// struct OnlyComponentAccessWorld<'w>(UnsafeWorldCell<'w>);
///
/// impl<'w> OnlyResourceAccessWorld<'w> {
///     fn get_resource_mut<T: Resource>(&mut self) -> Option<Mut<'w, T>> {
///         // SAFETY: resource access is allowed through this UnsafeWorldCell
///         unsafe { self.0.get_resource_mut::<T>() }
///     }
/// }
/// // impl<'w> OnlyComponentAccessWorld<'w> {
/// //     ...
/// // }
///
/// // the two `UnsafeWorldCell`s borrow from the `&mut World`, so it cannot be accessed while they are live
/// fn split_world_access(world: &mut World) -> (OnlyResourceAccessWorld<'_>, OnlyComponentAccessWorld<'_>) {
///     let unsafe_world_cell = world.as_unsafe_world_cell();
///     let resource_access = OnlyResourceAccessWorld(unsafe_world_cell);
///     let component_access = OnlyComponentAccessWorld(unsafe_world_cell);
///     (resource_access, component_access)
/// }
/// ```
#[derive(Copy, Clone)]
pub struct UnsafeWorldCell<'w>(*mut World, PhantomData<(&'w World, &'w UnsafeCell<World>)>);

// SAFETY: `&World` and `&mut World` are both `Send`
unsafe impl Send for UnsafeWorldCell<'_> {}
// SAFETY: `&World` and `&mut World` are both `Sync`
unsafe impl Sync for UnsafeWorldCell<'_> {}

impl<'w> UnsafeWorldCell<'w> {
    /// Creates a [`UnsafeWorldCell`] that can be used to access everything immutably
    pub(crate) fn new_readonly(world: &'w World) -> Self {
        UnsafeWorldCell(world as *const World as *mut World, PhantomData)
    }

    /// Creates [`UnsafeWorldCell`] that can be used to access everything mutably
    pub(crate) fn new_mutable(world: &'w mut World) -> Self {
        Self(world as *mut World, PhantomData)
    }

    /// This `&mut World` counts as a mutable borrow of every resource and component for the purposes
    /// of safety invariants that talk about borrows on component/resource data not existing. This is
    /// true even for methods that do not explicitly call out `&mut World` as problematic.
    ///
    /// # Safety
    /// - must have permission to access everything mutably
    /// - there must be no other borrows on world
    /// - returned borrow must not be used in any way if the world is accessed
    ///   through other means than the `&mut World` after this call.
    pub unsafe fn world_mut(self) -> &'w mut World {
        // SAFETY:
        // - caller ensures the created `&mut World` is the only borrow of world
        unsafe { &mut *self.0 }
    }

    /// Gets a reference to the [`&World`](crate::world::World) this [`UnsafeWorldCell`] belongs to.
    /// This can be used for arbitrary shared/readonly access.
    ///
    /// # Safety
    /// - must have permission to access the whole world immutably
    /// - there must be no live exclusive borrows on world data
    /// - there must be no live exclusive borrow of world
    pub unsafe fn world(self) -> &'w World {
        // SAFETY:
        // - caller ensures there is no `&mut World` this makes it okay to make a `&World`
        // - caller ensures there is no mutable borrows of world data, this means the caller cannot
        //   misuse the returned `&World`
        unsafe { &*self.0 }
    }

    /// Gets a reference to the [`World`] this [`UnsafeWorldCell`] belong to.
    /// This can be used for arbitrary read only access of world metadata
    ///
    /// You should attempt to use various safe methods on [`UnsafeWorldCell`] for
    /// metadata access before using this method.
    ///
    /// # Safety
    /// - must only be used to access world metadata
    pub unsafe fn world_metadata(self) -> &'w World {
        // SAFETY: caller ensures that returned reference is not used to violate aliasing rules
        unsafe { self.unsafe_world() }
    }

    /// Variant on [`UnsafeWorldCell::world`] solely used for implementing this type's methods.
    /// It allows having an `&World` even with live mutable borrows of components and resources
    /// so the returned `&World` should not be handed out to safe code and care should be taken
    /// when working with it.
    ///
    /// Deliberately private as the correct way to access data in a [`World`] that may have existing
    /// mutable borrows of data inside it, is to use [`UnsafeWorldCell`].
    ///
    /// # Safety
    /// - must not be used in a way that would conflict with any
    ///   live exclusive borrows on world data
    unsafe fn unsafe_world(self) -> &'w World {
        // SAFETY:
        // - caller ensures that the returned `&World` is not used in a way that would conflict
        //   with any existing mutable borrows of world data
        unsafe { &*self.0 }
    }

    /// Retrieves this world's [Entities] collection
    #[inline]
    pub fn entities(self) -> &'w Entities {
        // SAFETY:
        // - we only access world metadata
        &unsafe { self.world_metadata() }.entities
    }

    /// Retrieves this world's [Archetypes] collection
    #[inline]
    pub fn archetypes(self) -> &'w Archetypes {
        // SAFETY:
        // - we only access world metadata
        &unsafe { self.world_metadata() }.archetypes
    }

    /// Retrieves this world's [Components] collection
    #[inline]
    pub fn components(self) -> &'w Components {
        // SAFETY:
        // - we only access world metadata
        &unsafe { self.world_metadata() }.components
    }

    /// Retrieves this world's [Bundles] collection
    #[inline]
    pub fn bundles(self) -> &'w Bundles {
        // SAFETY:
        // - we only access world metadata
        &unsafe { self.world_metadata() }.bundles
    }

    /// Reads the current change tick of this world.
    #[inline]
    pub fn read_change_tick(self) -> u32 {
        // SAFETY:
        // - we only access world metadata
        unsafe { self.world_metadata() }
            .change_tick
            .load(Ordering::Acquire)
    }

    #[inline]
    pub fn last_change_tick(self) -> u32 {
        // SAFETY:
        // - we only access world metadata
        unsafe { self.world_metadata() }.last_change_tick
    }

    #[inline]
    pub fn increment_change_tick(self) -> u32 {
        // SAFETY:
        // - we only access world metadata
        unsafe { self.world_metadata() }
            .change_tick
            .fetch_add(1, Ordering::AcqRel)
    }

    /// Shorthand helper function for getting the [`ArchetypeComponentId`] for a resource.
    #[inline]
    pub(crate) fn get_resource_archetype_component_id(
        self,
        component_id: ComponentId,
    ) -> Option<ArchetypeComponentId> {
        // SAFETY:
        // - we only access world metadata
        let resource = unsafe { self.world_metadata() }
            .storages
            .resources
            .get(component_id)?;
        Some(resource.id())
    }

    /// Shorthand helper function for getting the [`ArchetypeComponentId`] for a resource.
    #[inline]
    pub(crate) fn get_non_send_archetype_component_id(
        self,
        component_id: ComponentId,
    ) -> Option<ArchetypeComponentId> {
        // SAFETY:
        // - we only access world metadata
        let resource = unsafe { self.world_metadata() }
            .storages
            .non_send_resources
            .get(component_id)?;
        Some(resource.id())
    }

    /// Retrieves an [`UnsafeEntityCell`] that exposes read and write operations for the given `entity`.
    /// Similar to the [`UnsafeWorldCell`], you are in charge of making sure that no aliasing rules are violated.
    pub fn get_entity(self, entity: Entity) -> Option<UnsafeEntityCell<'w>> {
        let location = self.entities().get(entity)?;
        Some(UnsafeEntityCell::new(self, entity, location))
    }

    /// Gets a reference to the resource of the given type if it exists
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource
    /// - no mutable reference to the resource exists at the same time
    #[inline]
    pub unsafe fn get_resource<R: Resource>(self) -> Option<&'w R> {
        let component_id = self.components().get_resource_id(TypeId::of::<R>())?;
        // SAFETY: caller ensures `self` has permission to access the resource
        //  caller also ensure that no mutable reference to the resource exists
        unsafe {
            self.get_resource_by_id(component_id)
                // SAFETY: `component_id` was obtained from the type ID of `R`.
                .map(|ptr| ptr.deref::<R>())
        }
    }

    /// Gets a pointer to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer must not be used to modify the resource, and must not be
    /// dereferenced after the borrow of the [`World`] ends.
    ///
    /// **You should prefer to use the typed API [`UnsafeWorldCell::get_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource
    /// - no mutable reference to the resource exists at the same time
    #[inline]
    pub unsafe fn get_resource_by_id(self, component_id: ComponentId) -> Option<Ptr<'w>> {
        // SAFETY: caller ensures that `self` has permission to access `R`
        //  caller ensures that no mutable reference exists to `R`
        unsafe { self.unsafe_world() }
            .storages
            .resources
            .get(component_id)?
            .get_data()
    }

    /// Gets a reference to the non-send resource of the given type if it exists
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource
    /// - no mutable reference to the resource exists at the same time
    #[inline]
    pub unsafe fn get_non_send_resource<R: 'static>(self) -> Option<&'w R> {
        let component_id = self.components().get_resource_id(TypeId::of::<R>())?;
        // SAFETY: caller ensures that `self` has permission to access `R`
        //  caller ensures that no mutable reference exists to `R`
        unsafe {
            self.get_non_send_resource_by_id(component_id)
                // SAFETY: `component_id` was obtained from `TypeId::of::<R>()`
                .map(|ptr| ptr.deref::<R>())
        }
    }

    /// Gets a `!Send` resource to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer must not be used to modify the resource, and must not be
    /// dereferenced after the immutable borrow of the [`World`] ends.
    ///
    /// **You should prefer to use the typed API [`UnsafeWorldCell::get_non_send_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Panics
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource
    /// - no mutable reference to the resource exists at the same time
    #[inline]
    pub unsafe fn get_non_send_resource_by_id(self, component_id: ComponentId) -> Option<Ptr<'w>> {
        // SAFETY: we only access data on world that the caller has ensured is unaliased and we have
        //  permission to access.
        unsafe { self.unsafe_world() }
            .storages
            .non_send_resources
            .get(component_id)?
            .get_data()
    }

    /// Gets a mutable reference to the resource of the given type if it exists
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource mutably
    /// - no other references to the resource exist at the same time
    #[inline]
    pub unsafe fn get_resource_mut<R: Resource>(self) -> Option<Mut<'w, R>> {
        let component_id = self.components().get_resource_id(TypeId::of::<R>())?;
        // SAFETY:
        // - caller ensures `self` has permission to access the resource mutably
        // - caller ensures no other references to the resource exist
        unsafe {
            self.get_resource_mut_by_id(component_id)
                // `component_id` was gotten from `TypeId::of::<R>()`
                .map(|ptr| ptr.with_type::<R>())
        }
    }

    /// Gets a pointer to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer may be used to modify the resource, as long as the mutable borrow
    /// of the [`UnsafeWorldCell`] is still valid.
    ///
    /// **You should prefer to use the typed API [`UnsafeWorldCell::get_resource_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource mutably
    /// - no other references to the resource exist at the same time
    #[inline]
    pub unsafe fn get_resource_mut_by_id(
        self,
        component_id: ComponentId,
    ) -> Option<MutUntyped<'w>> {
        // SAFETY: we only access data that the caller has ensured is unaliased and `self`
        //  has permission to access.
        let (ptr, ticks) = unsafe { self.unsafe_world() }
            .storages
            .resources
            .get(component_id)?
            .get_with_ticks()?;

        // SAFETY:
        // - index is in-bounds because the column is initialized and non-empty
        // - the caller promises that no other reference to the ticks of the same row can exist at the same time
        let ticks = unsafe {
            TicksMut::from_tick_cells(ticks, self.last_change_tick(), self.read_change_tick())
        };

        Some(MutUntyped {
            // SAFETY: This function has exclusive access to the world so nothing aliases `ptr`.
            value: unsafe { ptr.assert_unique() },
            ticks,
        })
    }

    /// Gets a mutable reference to the non-send resource of the given type if it exists
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource mutably
    /// - no other references to the resource exist at the same time
    #[inline]
    pub unsafe fn get_non_send_resource_mut<R: 'static>(self) -> Option<Mut<'w, R>> {
        let component_id = self.components().get_resource_id(TypeId::of::<R>())?;
        // SAFETY:
        // - caller ensures that `self` has permission to access the resource
        // - caller ensures that the resource is unaliased
        unsafe {
            self.get_non_send_resource_mut_by_id(component_id)
                // SAFETY: `component_id` was gotten by `TypeId::of::<R>()`
                .map(|ptr| ptr.with_type::<R>())
        }
    }

    /// Gets a `!Send` resource to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer may be used to modify the resource, as long as the mutable borrow
    /// of the [`World`] is still valid.
    ///
    /// **You should prefer to use the typed API [`UnsafeWorldCell::get_non_send_resource_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Panics
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource mutably
    /// - no other references to the resource exist at the same time
    #[inline]
    pub unsafe fn get_non_send_resource_mut_by_id(
        self,
        component_id: ComponentId,
    ) -> Option<MutUntyped<'w>> {
        let change_tick = self.read_change_tick();
        // SAFETY: we only access data that the caller has ensured is unaliased and `self`
        //  has permission to access.
        let (ptr, ticks) = unsafe { self.unsafe_world() }
            .storages
            .non_send_resources
            .get(component_id)?
            .get_with_ticks()?;

        let ticks =
            // SAFETY: This function has exclusive access to the world so nothing aliases `ticks`.
            // - index is in-bounds because the column is initialized and non-empty
            // - no other reference to the ticks of the same row can exist at the same time
            unsafe { TicksMut::from_tick_cells(ticks, self.last_change_tick(), change_tick) };

        Some(MutUntyped {
            // SAFETY: This function has exclusive access to the world so nothing aliases `ptr`.
            value: unsafe { ptr.assert_unique() },
            ticks,
        })
    }

    // Shorthand helper function for getting the data and change ticks for a resource.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource mutably
    /// - no mutable references to the resource exist at the same time
    #[inline]
    pub(crate) unsafe fn get_resource_with_ticks(
        self,
        component_id: ComponentId,
    ) -> Option<(Ptr<'w>, TickCells<'w>)> {
        // SAFETY:
        // - caller ensures there is no `&mut World`
        // - caller ensures there are no mutable borrows of this resource
        // - caller ensures that we have permission to access this resource
        unsafe { self.unsafe_world() }
            .storages
            .resources
            .get(component_id)?
            .get_with_ticks()
    }

    // Shorthand helper function for getting the data and change ticks for a resource.
    ///
    /// # Panics
    /// This function will panic if it isn't called from the same thread that the resource was inserted from.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource mutably
    /// - no mutable references to the resource exist at the same time
    #[inline]
    pub(crate) unsafe fn get_non_send_with_ticks(
        self,
        component_id: ComponentId,
    ) -> Option<(Ptr<'w>, TickCells<'w>)> {
        // SAFETY:
        // - caller ensures there is no `&mut World`
        // - caller ensures there are no mutable borrows of this resource
        // - caller ensures that we have permission to access this resource
        unsafe { self.unsafe_world() }
            .storages
            .non_send_resources
            .get(component_id)?
            .get_with_ticks()
    }
}

/// A interior-mutable reference to a particular [`Entity`] and all of its components
#[derive(Copy, Clone)]
pub struct UnsafeEntityCell<'w> {
    world: UnsafeWorldCell<'w>,
    entity: Entity,
    location: EntityLocation,
}

impl<'w> UnsafeEntityCell<'w> {
    pub(crate) fn new(
        world: UnsafeWorldCell<'w>,
        entity: Entity,
        location: EntityLocation,
    ) -> Self {
        UnsafeEntityCell {
            world,
            entity,
            location,
        }
    }

    #[inline]
    #[must_use = "Omit the .id() call if you do not need to store the `Entity` identifier."]
    pub fn id(self) -> Entity {
        self.entity
    }

    #[inline]
    pub fn location(self) -> EntityLocation {
        self.location
    }

    #[inline]
    pub fn archetype(self) -> &'w Archetype {
        &self.world.archetypes()[self.location.archetype_id]
    }

    #[inline]
    pub fn world(self) -> UnsafeWorldCell<'w> {
        self.world
    }

    #[inline]
    pub fn contains<T: Component>(self) -> bool {
        self.contains_type_id(TypeId::of::<T>())
    }

    #[inline]
    pub fn contains_id(self, component_id: ComponentId) -> bool {
        self.archetype().contains(component_id)
    }

    #[inline]
    pub fn contains_type_id(self, type_id: TypeId) -> bool {
        let id = match self.world.components().get_id(type_id) {
            Some(id) => id,
            None => return false,
        };
        self.contains_id(id)
    }

    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeEntityCell`] has permission to access the component
    /// - no other mutable references to the component exist at the same time
    #[inline]
    pub unsafe fn get<T: Component>(self) -> Option<&'w T> {
        let component_id = self.world.components().get_id(TypeId::of::<T>())?;

        // SAFETY:
        // - entity location is valid
        // - proper world access is promised by caller
        unsafe {
            get_component(
                self.world,
                component_id,
                T::Storage::STORAGE_TYPE,
                self.entity,
                self.location,
            )
            // SAFETY: returned component is of type T
            .map(|value| value.deref::<T>())
        }
    }

    /// Retrieves the change ticks for the given component. This can be useful for implementing change
    /// detection in custom runtimes.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeEntityCell`] has permission to access the component
    /// - no other mutable references to the component exist at the same time
    #[inline]
    pub unsafe fn get_change_ticks<T: Component>(self) -> Option<ComponentTicks> {
        let component_id = self.world.components().get_id(TypeId::of::<T>())?;

        // SAFETY:
        // - entity location is valid
        // - proper world access is promised by caller
        unsafe {
            get_ticks(
                self.world,
                component_id,
                T::Storage::STORAGE_TYPE,
                self.entity,
                self.location,
            )
        }
    }

    /// Retrieves the change ticks for the given [`ComponentId`]. This can be useful for implementing change
    /// detection in custom runtimes.
    ///
    /// **You should prefer to use the typed API [`UnsafeEntityCell::get_change_ticks`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeEntityCell`] has permission to access the component
    /// - no other mutable references to the component exist at the same time
    #[inline]
    pub unsafe fn get_change_ticks_by_id(
        &self,
        component_id: ComponentId,
    ) -> Option<ComponentTicks> {
        let info = self.world.components().get_info(component_id)?;
        // SAFETY:
        // - entity location and entity is valid
        // - world access is immutable, lifetime tied to `&self`
        // - the storage type provided is correct for T
        unsafe {
            get_ticks(
                self.world,
                component_id,
                info.storage_type(),
                self.entity,
                self.location,
            )
        }
    }

    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeEntityCell`] has permission to access the component mutably
    /// - no other references to the component exist at the same time
    #[inline]
    pub unsafe fn get_mut<T: Component>(self) -> Option<Mut<'w, T>> {
        // SAFETY: same safety requirements
        unsafe {
            self.get_mut_using_ticks(self.world.last_change_tick(), self.world.read_change_tick())
        }
    }

    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeEntityCell`] has permission to access the component mutably
    /// - no other references to the component exist at the same time
    #[inline]
    pub(crate) unsafe fn get_mut_using_ticks<T: Component>(
        &self,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Option<Mut<'w, T>> {
        let component_id = self.world.components().get_id(TypeId::of::<T>())?;

        // SAFETY:
        // - `storage_type` is correct
        // - `location` is valid
        // - aliasing rules are ensured by caller
        unsafe {
            get_component_and_ticks(
                self.world,
                component_id,
                T::Storage::STORAGE_TYPE,
                self.entity,
                self.location,
            )
            .map(|(value, cells)| Mut {
                value: value.assert_unique().deref_mut::<T>(),
                ticks: TicksMut::from_tick_cells(cells, last_change_tick, change_tick),
            })
        }
    }
}

impl<'w> UnsafeEntityCell<'w> {
    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`UnsafeEntityCell::get`], this returns a raw pointer to the component,
    /// which is only valid while the `'w` borrow of the lifetime is active.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeEntityCell`] has permission to access the component
    /// - no other mutable references to the component exist at the same time
    #[inline]
    pub unsafe fn get_by_id(self, component_id: ComponentId) -> Option<Ptr<'w>> {
        let info = self.world.components().get_info(component_id)?;
        // SAFETY: entity_location is valid, component_id is valid as checked by the line above
        unsafe {
            get_component(
                self.world,
                component_id,
                info.storage_type(),
                self.entity,
                self.location,
            )
        }
    }

    /// Retrieves a mutable untyped reference to the given `entity`'s [Component] of the given [`ComponentId`].
    /// Returns [None] if the `entity` does not have a [Component] of the given type.
    ///
    /// **You should prefer to use the typed API [`UnsafeEntityCell::get_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeEntityCell`] has permission to access the component mutably
    /// - no other references to the component exist at the same time
    #[inline]
    pub unsafe fn get_mut_by_id(self, component_id: ComponentId) -> Option<MutUntyped<'w>> {
        let info = self.world.components().get_info(component_id)?;
        // SAFETY: entity_location is valid, component_id is valid as checked by the line above
        unsafe {
            get_component_and_ticks(
                self.world,
                component_id,
                info.storage_type(),
                self.entity,
                self.location,
            )
            .map(|(value, cells)| MutUntyped {
                // SAFETY: world access validated by caller and ties world lifetime to `MutUntyped` lifetime
                value: value.assert_unique(),
                ticks: TicksMut::from_tick_cells(
                    cells,
                    self.world.last_change_tick(),
                    self.world.read_change_tick(),
                ),
            })
        }
    }
}

impl<'w> UnsafeWorldCell<'w> {
    #[inline]
    /// # Safety:
    /// - the returned `Column` is only used in ways that this [`UnsafeWorldCell`] has permission for.
    /// - the returned `Column` is only used in ways that would not conflict with any existing
    ///   borrows of world data.
    unsafe fn fetch_table(
        self,
        location: EntityLocation,
        component_id: ComponentId,
    ) -> Option<&'w Column> {
        // SAFETY: caller ensures returned data is not misused and we have not created any borrows
        // of component/resource data
        unsafe { self.unsafe_world() }.storages.tables[location.table_id].get_column(component_id)
    }

    #[inline]
    /// # Safety:
    /// - the returned `ComponentSparseSet` is only used in ways that this [`UnsafeWorldCell`] has permission for.
    /// - the returned `ComponentSparseSet` is only used in ways that would not conflict with any existing
    ///   borrows of world data.
    unsafe fn fetch_sparse_set(self, component_id: ComponentId) -> Option<&'w ComponentSparseSet> {
        // SAFETY: caller ensures returned data is not misused and we have not created any borrows
        // of component/resource data
        unsafe { self.unsafe_world() }
            .storages
            .sparse_sets
            .get(component_id)
    }
}

/// Get an untyped pointer to a particular [`Component`](crate::component::Component) on a particular [`Entity`] in the provided [`World`](crate::world::World).
///
/// # Safety
/// - `location` must refer to an archetype that contains `entity`
/// the archetype
/// - `component_id` must be valid
/// - `storage_type` must accurately reflect where the components for `component_id` are stored.
/// - the caller must ensure that no aliasing rules are violated
#[inline]
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn get_component(
    world: UnsafeWorldCell<'_>,
    component_id: ComponentId,
    storage_type: StorageType,
    entity: Entity,
    location: EntityLocation,
) -> Option<Ptr<'_>> {
    // SAFETY: component_id exists and is therefore valid
    match storage_type {
        StorageType::Table => {
            let components = world.fetch_table(location, component_id)?;
            // SAFETY: archetypes only store valid table_rows and caller ensure aliasing rules
            Some(components.get_data_unchecked(location.table_row))
        }
        StorageType::SparseSet => world.fetch_sparse_set(component_id)?.get(entity),
    }
}

/// Get an untyped pointer to a particular [`Component`](crate::component::Component) and its [`ComponentTicks`]
///
/// # Safety
/// - `location` must refer to an archetype that contains `entity`
/// - `component_id` must be valid
/// - `storage_type` must accurately reflect where the components for `component_id` are stored.
/// - the caller must ensure that no aliasing rules are violated
#[inline]
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn get_component_and_ticks(
    world: UnsafeWorldCell<'_>,
    component_id: ComponentId,
    storage_type: StorageType,
    entity: Entity,
    location: EntityLocation,
) -> Option<(Ptr<'_>, TickCells<'_>)> {
    match storage_type {
        StorageType::Table => {
            let components = world.fetch_table(location, component_id)?;

            // SAFETY: archetypes only store valid table_rows and caller ensure aliasing rules
            Some((
                components.get_data_unchecked(location.table_row),
                TickCells {
                    added: components.get_added_ticks_unchecked(location.table_row),
                    changed: components.get_changed_ticks_unchecked(location.table_row),
                },
            ))
        }
        StorageType::SparseSet => world.fetch_sparse_set(component_id)?.get_with_ticks(entity),
    }
}

/// Get an untyped pointer to the [`ComponentTicks`] on a particular [`Entity`]
///
/// # Safety
/// - `location` must refer to an archetype that contains `entity`
/// the archetype
/// - `component_id` must be valid
/// - `storage_type` must accurately reflect where the components for `component_id` are stored.
/// - the caller must ensure that no aliasing rules are violated
#[inline]
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn get_ticks(
    world: UnsafeWorldCell<'_>,
    component_id: ComponentId,
    storage_type: StorageType,
    entity: Entity,
    location: EntityLocation,
) -> Option<ComponentTicks> {
    match storage_type {
        StorageType::Table => {
            let components = world.fetch_table(location, component_id)?;
            // SAFETY: archetypes only store valid table_rows and caller ensure aliasing rules
            Some(components.get_ticks_unchecked(location.table_row))
        }
        StorageType::SparseSet => world.fetch_sparse_set(component_id)?.get_ticks(entity),
    }
}
