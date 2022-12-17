#![warn(unsafe_op_in_unsafe_fn)]

use super::{Mut, World};
use crate::{
    archetype::Archetypes,
    bundle::Bundles,
    change_detection::{MutUntyped, Ticks},
    component::{ComponentId, Components},
    entity::Entities,
    storage::Storages,
    system::Resource,
};
use bevy_ptr::Ptr;
use std::any::TypeId;

/// Variant of the [`World`] where resource and component accesses takes a `&World`, and the responsibility to avoid
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
/// [`InteriorMutableWorld`] can be used as a building block for writing APIs that safely allow disjoint access into the world.
/// In the following example, the world is split into a resource access half and a component access half, where each one can
/// safely hand out mutable references.
///
/// ```
/// use bevy_ecs::world::World;
/// use bevy_ecs::change_detection::Mut;
/// use bevy_ecs::system::Resource;
/// use bevy_ecs::world::interior_mutable_world::InteriorMutableWorld;
///
/// // INVARIANT: existance of this struct means that users of it are the only ones being able to access resources in the world
/// struct OnlyResourceAccessWorld<'w>(InteriorMutableWorld<'w>);
/// // INVARIANT: existance of this struct means that users of it are the only ones being able to access components in the world
/// struct OnlyComponentAccessWorld<'w>(InteriorMutableWorld<'w>);
///
/// impl<'w> OnlyResourceAccessWorld<'w> {
///     fn get_resource_mut<T: Resource>(&mut self) -> Option<Mut<'w, T>> {
///         // SAFETY: resource access is allowed through this InteriorMutableWorld
///         unsafe { self.0.get_resource_mut::<T>() }
///     }
/// }
/// // impl<'w> OnlyComponentAccessWorld<'w> {
/// //     ...
/// // }
///
/// // the two interior mutable worlds borrow from the `&mut World`, so it cannot be accessed while they are live
/// fn split_world_access(world: &mut World) -> (OnlyResourceAccessWorld<'_>, OnlyComponentAccessWorld<'_>) {
///     let resource_access = OnlyResourceAccessWorld(unsafe { world.as_interior_mutable() });
///     let component_access = OnlyComponentAccessWorld(unsafe { world.as_interior_mutable() });
///     (resource_access, component_access)
/// }
/// ```
#[derive(Copy, Clone)]
pub struct InteriorMutableWorld<'w>(&'w World);

impl<'w> InteriorMutableWorld<'w> {
    pub(crate) fn new(world: &'w World) -> Self {
        InteriorMutableWorld(world)
    }

    /// Gets a reference to the [`&World`](crate::world::World) this [`InteriorMutableWorld`] belongs to.
    /// This can be used to call methods like [`World::read_change_tick`] which aren't exposed here but don't perform any accesses.
    ///
    /// **Note**: You *must not* hand out a `&World` reference to arbitrary safe code when the [`InteriorMutableWorld`] is currently
    /// being used for mutable accesses.
    ///
    /// SAFETY:
    /// - the world must not be used to access any resources or components. You can use it to safely access metadata.
    pub unsafe fn world(&self) -> &'w World {
        self.0
    }

    /// Retrieves this world's [Entities] collection
    #[inline]
    pub fn entities(&self) -> &'w Entities {
        &self.0.entities
    }

    /// Retrieves this world's [Archetypes] collection
    #[inline]
    pub fn archetypes(&self) -> &'w Archetypes {
        &self.0.archetypes
    }

    /// Retrieves this world's [Components] collection
    #[inline]
    pub fn components(&self) -> &'w Components {
        &self.0.components
    }

    /// Retrieves this world's [Storages] collection
    #[inline]
    pub fn storages(&self) -> &'w Storages {
        &self.0.storages
    }

    /// Retrieves this world's [Bundles] collection
    #[inline]
    pub fn bundles(&self) -> &'w Bundles {
        &self.0.bundles
    }
    /// Gets a reference to the resource of the given type if it exists
    ///
    /// # Safety
    /// All [`InteriorMutableWorld`] methods take `&self` and thus do not check that there is only one unique reference or multiple shared ones.
    /// It is the callers responsibility to make sure that there will never be a mutable reference to a value that has other references pointing to it,
    /// and that no arbitrary safe code can access a `&World` while some value is mutably borrowed.
    #[inline]
    pub unsafe fn get_resource<R: Resource>(&self) -> Option<&'w R> {
        self.0.get_resource::<R>()
    }

    /// Gets a pointer to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer must not be used to modify the resource, and must not be
    /// dereferenced after the borrow of the [`World`] ends.
    ///
    /// **You should prefer to use the typed API [`InteriorMutableWorld::get_resource`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Safety
    /// All [`InteriorMutableWorld`] methods take `&self` and thus do not check that there is only one unique reference or multiple shared ones.
    /// It is the callers responsibility to make sure that there will never be a mutable reference to a value that has other references pointing to it,
    /// and that no arbitrary safe code can access a `&World` while some value is mutably borrowed.
    #[inline]
    pub unsafe fn get_resource_by_id(&self, component_id: ComponentId) -> Option<Ptr<'w>> {
        self.0.get_resource_by_id(component_id)
    }

    /// Gets a mutable reference to the resource of the given type if it exists
    ///
    /// # Safety
    /// All [`InteriorMutableWorld`] methods take `&self` and thus do not check that there is only one unique reference or multiple shared ones.
    /// It is the callers responsibility to make sure that there will never be a mutable reference to a value that has other references pointing to it,
    /// and that no arbitrary safe code can access a `&World` while some value is mutably borrowed.
    #[inline]
    pub unsafe fn get_resource_mut<R: Resource>(&self) -> Option<Mut<'w, R>> {
        let component_id = self.0.components.get_resource_id(TypeId::of::<R>())?;
        // SAFETY:
        // - component_id is of type `R`
        // - caller ensures aliasing rules
        // - `R` is Send + Sync
        unsafe { self.0.get_resource_unchecked_mut_with_id(component_id) }
    }

    /// Gets a pointer to the resource with the id [`ComponentId`] if it exists.
    /// The returned pointer may be used to modify the resource, as long as the mutable borrow
    /// of the [`InteriorMutableWorld`] is still valid.
    ///
    /// **You should prefer to use the typed API [`InteriorMutableWorld::get_resource_mut`] where possible and only
    /// use this in cases where the actual types are not known at compile time.**
    ///
    /// # Safety
    /// All [`InteriorMutableWorld`] methods take `&self` and thus do not check that there is only one unique reference or multiple shared ones.
    /// It is the callers responsibility to make sure that there will never be a mutable reference to a value that has other references pointing to it,
    /// and that no arbitrary safe code can access a `&World` while some value is mutably borrowed.
    #[inline]
    pub unsafe fn get_resource_mut_by_id(
        &self,
        component_id: ComponentId,
    ) -> Option<MutUntyped<'w>> {
        let info = self.0.components.get_info(component_id)?;
        if !info.is_send_and_sync() {
            self.0.validate_non_send_access_untyped(info.name());
        }

        let (ptr, ticks) = self.0.get_resource_with_ticks(component_id)?;

        // SAFETY:
        // - index is in-bounds because the column is initialized and non-empty
        // - the caller promises that no other reference to the ticks of the same row can exist at the same time
        let ticks = unsafe {
            Ticks::from_tick_cells(ticks, self.0.last_change_tick(), self.0.read_change_tick())
        };

        Some(MutUntyped {
            // SAFETY: This function has exclusive access to the world so nothing aliases `ptr`.
            value: unsafe { ptr.assert_unique() },
            ticks,
        })
    }

    /// Gets a reference to the non-send resource of the given type, if it exists.
    /// Otherwise returns [None]
    #[inline]
    pub unsafe fn get_non_send_resource<R: 'static>(&self) -> Option<&R> {
        self.0.get_non_send_resource::<R>()
    }
    /// Gets a mutable reference to the non-send resource of the given type, if it exists.
    /// Otherwise returns [None]
    #[inline]
    pub unsafe fn get_non_send_resource_mut<R: 'static>(&mut self) -> Option<Mut<'w, R>> {
        let component_id = self.0.components.get_resource_id(TypeId::of::<R>())?;
        // SAFETY: safety requirement is deferred to the caller
        unsafe { self.0.get_non_send_unchecked_mut_with_id(component_id) }
    }
}
