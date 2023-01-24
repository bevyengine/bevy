#![warn(unsafe_op_in_unsafe_fn)]

use super::{entity_ref, Mut, World};
use crate::{
    archetype::{Archetype, Archetypes},
    bundle::Bundles,
    change_detection::{MutUntyped, TicksMut},
    component::{ComponentId, ComponentStorage, ComponentTicks, Components},
    entity::{Entities, Entity, EntityLocation},
    prelude::Component,
    storage::Storages,
    system::Resource,
};
use bevy_ptr::Ptr;
use std::any::TypeId;

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
pub struct UnsafeWorldCell<'w>(&'w World);

impl<'w> UnsafeWorldCell<'w> {
    pub(crate) fn new(world: &'w World) -> Self {
        UnsafeWorldCell(world)
    }

    /// Gets a reference to the [`&World`](crate::world::World) this [`UnsafeWorldCell`] belongs to.
    /// This can be used to call methods like [`World::contains_resource`] which aren't exposed and but don't perform any accesses.
    ///
    /// **Note**: You *must not* hand out a `&World` reference to arbitrary safe code when the [`UnsafeWorldCell`] is currently
    /// being used for mutable accesses.
    ///
    /// # Safety
    /// - the world must not be used to access any resources or components. You can use it to safely access metadata.
    pub unsafe fn world(self) -> &'w World {
        self.0
    }

    /// Retrieves this world's [Entities] collection
    #[inline]
    pub fn entities(self) -> &'w Entities {
        &self.0.entities
    }

    /// Retrieves this world's [Archetypes] collection
    #[inline]
    pub fn archetypes(self) -> &'w Archetypes {
        &self.0.archetypes
    }

    /// Retrieves this world's [Components] collection
    #[inline]
    pub fn components(self) -> &'w Components {
        &self.0.components
    }

    /// Retrieves this world's [Storages] collection
    #[inline]
    pub fn storages(self) -> &'w Storages {
        &self.0.storages
    }

    /// Retrieves this world's [Bundles] collection
    #[inline]
    pub fn bundles(self) -> &'w Bundles {
        &self.0.bundles
    }

    /// Reads the current change tick of this world.
    #[inline]
    pub fn read_change_tick(self) -> u32 {
        self.0.read_change_tick()
    }

    #[inline]
    pub fn last_change_tick(self) -> u32 {
        self.0.last_change_tick()
    }

    #[inline]
    pub fn increment_change_tick(self) -> u32 {
        self.0.increment_change_tick()
    }

    /// Retrieves an [`UnsafeWorldCellEntityRef`] that exposes read and write operations for the given `entity`.
    /// Similar to the [`UnsafeWorldCell`], you are in charge of making sure that no aliasing rules are violated.
    pub fn get_entity(self, entity: Entity) -> Option<UnsafeWorldCellEntityRef<'w>> {
        let location = self.0.entities.get(entity)?;
        Some(UnsafeWorldCellEntityRef::new(self, entity, location))
    }

    /// Gets a reference to the resource of the given type if it exists
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource
    /// - no mutable reference to the resource exists at the same time
    #[inline]
    pub unsafe fn get_resource<R: Resource>(self) -> Option<&'w R> {
        self.0.get_resource::<R>()
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
        self.0.get_resource_by_id(component_id)
    }

    /// Gets a reference to the non-send resource of the given type if it exists
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCell`] has permission to access the resource
    /// - no mutable reference to the resource exists at the same time
    #[inline]
    pub unsafe fn get_non_send_resource<R: 'static>(self) -> Option<&'w R> {
        self.0.get_non_send_resource()
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
        self.0
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
        let component_id = self.0.components.get_resource_id(TypeId::of::<R>())?;
        // SAFETY:
        // - component_id is of type `R`
        // - caller ensures aliasing rules
        // - `R` is Send + Sync
        unsafe { self.get_resource_mut_with_id(component_id) }
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
        &self,
        component_id: ComponentId,
    ) -> Option<MutUntyped<'w>> {
        let (ptr, ticks) = self.0.get_resource_with_ticks(component_id)?;

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
        let component_id = self.0.components.get_resource_id(TypeId::of::<R>())?;
        // SAFETY: component_id matches `R`, rest is promised by caller
        unsafe { self.get_non_send_mut_with_id(component_id) }
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
        &mut self,
        component_id: ComponentId,
    ) -> Option<MutUntyped<'_>> {
        let change_tick = self.read_change_tick();
        let (ptr, ticks) = self.0.get_non_send_with_ticks(component_id)?;

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
}

impl<'w> UnsafeWorldCell<'w> {
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - `component_id` must be assigned to a component of type `R`
    /// - the [`UnsafeWorldCell`] has permission to access the resource
    /// - no other mutable references to the resource exist at the same time
    #[inline]
    pub(crate) unsafe fn get_resource_mut_with_id<R>(
        &self,
        component_id: ComponentId,
    ) -> Option<Mut<'w, R>> {
        let (ptr, ticks) = self.0.get_resource_with_ticks(component_id)?;

        // SAFETY:
        // - This caller ensures that nothing aliases `ticks`.
        // - index is in-bounds because the column is initialized and non-empty
        let ticks = unsafe {
            TicksMut::from_tick_cells(ticks, self.last_change_tick(), self.read_change_tick())
        };

        Some(Mut {
            // SAFETY: caller ensures aliasing rules, ptr is of type `R`
            value: unsafe { ptr.assert_unique().deref_mut() },
            ticks,
        })
    }

    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - `component_id` must be assigned to a component of type `R`.
    /// - the [`UnsafeWorldCell`] has permission to access the resource mutably
    /// - no other references to the resource exist at the same time
    #[inline]
    pub(crate) unsafe fn get_non_send_mut_with_id<R: 'static>(
        &self,
        component_id: ComponentId,
    ) -> Option<Mut<'w, R>> {
        let (ptr, ticks) = self
            .0
            .storages
            .non_send_resources
            .get(component_id)?
            .get_with_ticks()?;
        Some(Mut {
            // SAFETY: caller ensures unique access
            value: unsafe { ptr.assert_unique().deref_mut() },
            // SAFETY: caller ensures unique access
            ticks: unsafe {
                TicksMut::from_tick_cells(ticks, self.last_change_tick(), self.read_change_tick())
            },
        })
    }
}

/// A interior-mutable reference to a particular [`Entity`] and all of its components
#[derive(Copy, Clone)]
pub struct UnsafeWorldCellEntityRef<'w> {
    world: UnsafeWorldCell<'w>,
    entity: Entity,
    location: EntityLocation,
}

impl<'w> UnsafeWorldCellEntityRef<'w> {
    pub(crate) fn new(
        world: UnsafeWorldCell<'w>,
        entity: Entity,
        location: EntityLocation,
    ) -> Self {
        UnsafeWorldCellEntityRef {
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
        &self.world.0.archetypes[self.location.archetype_id]
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
        entity_ref::contains_component_with_id(self.world.0, component_id, self.location)
    }

    #[inline]
    pub fn contains_type_id(self, type_id: TypeId) -> bool {
        entity_ref::contains_component_with_type(self.world.0, type_id, self.location)
    }

    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCellEntityRef`] has permission to access the component
    /// - no other mutable references to the component exist at the same time
    #[inline]
    pub unsafe fn get<T: Component>(self) -> Option<&'w T> {
        // SAFETY:
        // - entity location is valid
        // - proper world access is promised by caller
        unsafe {
            self.world
                .0
                .get_component_with_type(
                    TypeId::of::<T>(),
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
    /// - the [`UnsafeWorldCellEntityRef`] has permission to access the component
    /// - no other mutable references to the component exist at the same time
    #[inline]
    pub unsafe fn get_change_ticks<T: Component>(self) -> Option<ComponentTicks> {
        // SAFETY:
        // - entity location is valid
        // - proper world acess is promised by caller
        unsafe {
            self.world.0.get_ticks_with_type(
                TypeId::of::<T>(),
                T::Storage::STORAGE_TYPE,
                self.entity,
                self.location,
            )
        }
    }

    /// Retrieves the change ticks for the given [`ComponentId`]. This can be useful for implementing change
    /// detection in custom runtimes.
    ///
    /// **You should prefer to use the typed API [`UnsafeWorldCellEntityRef::get_change_ticks`] where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCellEntityRef`] has permission to access the component
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
            self.world.0.get_ticks(
                component_id,
                info.storage_type(),
                self.entity,
                self.location,
            )
        }
    }

    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCellEntityRef`] has permission to access the component mutably
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
    /// - the [`UnsafeWorldCellEntityRef`] has permission to access the component mutably
    /// - no other references to the component exist at the same time
    #[inline]
    pub(crate) unsafe fn get_mut_using_ticks<T: Component>(
        &self,
        last_change_tick: u32,
        change_tick: u32,
    ) -> Option<Mut<'w, T>> {
        // SAFETY:
        // - `storage_type` is correct
        // - `location` is valid
        // - aliasing rules are ensured by caller
        unsafe {
            self.world
                .0
                .get_component_and_ticks_with_type(
                    TypeId::of::<T>(),
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

impl<'w> UnsafeWorldCellEntityRef<'w> {
    /// Gets the component of the given [`ComponentId`] from the entity.
    ///
    /// **You should prefer to use the typed API where possible and only
    /// use this in cases where the actual component types are not known at
    /// compile time.**
    ///
    /// Unlike [`UnsafeWorldCellEntityRef::get`], this returns a raw pointer to the component,
    /// which is only valid while the `'w` borrow of the lifetime is active.
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCellEntityRef`] has permission to access the component
    /// - no other mutable references to the component exist at the same time
    #[inline]
    pub unsafe fn get_by_id(self, component_id: ComponentId) -> Option<Ptr<'w>> {
        let info = self.world.0.components.get_info(component_id)?;
        // SAFETY: entity_location is valid, component_id is valid as checked by the line above
        unsafe {
            self.world.0.get_component(
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
    /// **You should prefer to use the typed API [`UnsafeWorldCellEntityRef::get_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Safety
    /// It is the callers responsibility to ensure that
    /// - the [`UnsafeWorldCellEntityRef`] has permission to access the component mutably
    /// - no other references to the component exist at the same time
    #[inline]
    pub unsafe fn get_mut_by_id(self, component_id: ComponentId) -> Option<MutUntyped<'w>> {
        let info = self.world.0.components.get_info(component_id)?;
        // SAFETY: entity_location is valid, component_id is valid as checked by the line above
        unsafe {
            self.world
                .0
                .get_component_and_ticks(
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
