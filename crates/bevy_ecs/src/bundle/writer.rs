use crate::{
    component::{Component, ComponentId, ComponentsRegistrator},
    relationship::RelationshipHookMode,
    world::EntityWorldMut,
};
use alloc::vec::Vec;
use bevy_ptr::OwningPtr;
use bumpalo::Bump;
use core::ptr::NonNull;

/// Enables pushing components to internal scratch space (uses a bump allocator), which can then be
/// written as a dynamic bundle. The contents are cleared after each write and the allocated scratch
/// space is reused across writes.
#[derive(Default)]
pub struct BundleWriter {
    // Correctness: this should not be exposed, otherwise a caller could clear bundle_scratch, which
    // would ultimately result in Drop not being called on any of the items currently stored here
    bundle_scratch: OwnedBundleScratch,
    // Safety: this cannot be exposed, otherwise `alloc.reset()` could be called in arbitrary places,
    // which could invalidate the data stored in OwnedBundleScratch.
    alloc: Bump,
}

// SAFETY: The `NonNull` in `OwnedBundleScratch` is always a `Component`, which is Send
unsafe impl Send for BundleWriter {}

impl BundleWriter {
    /// Pushes the given component to the back of the current bundle scratch space. It will register
    /// the component in `components` if it does not already exist.
    ///
    /// # Safety
    ///
    /// `components` must be from the same world that all previous [`Self::push_component`] calls were called with,
    /// and the _next_  [`Self::write`] call.
    pub unsafe fn push_component<C: Component>(
        &mut self,
        components: &mut ComponentsRegistrator,
        component: C,
    ) {
        let id = components.register_component::<C>();
        let component_mut = self.alloc.alloc(component);
        // SAFETY: component_mut is not null, as it was allocated above with the `component` value
        let component_ptr =
            unsafe { NonNull::new_unchecked(core::ptr::from_mut(component_mut).cast::<u8>()) };
        // SAFETY: component id looked up above
        unsafe { self.bundle_scratch.push_ptr(id, component_ptr) };
    }

    /// Writes the current contents of the bundle to the given `entity` and clears the scratch space.
    ///
    /// # Safety
    ///
    /// `entity` must be from the same world that all [`Self::push_component`] calls since the last
    /// [`Self::write`] were called with.
    pub unsafe fn write(&mut self, entity: &mut EntityWorldMut) {
        // SAFETY: caller verifies that `entity` is from the same world as all previous `push_component` calls.
        unsafe { self.bundle_scratch.write(entity, RelationshipHookMode::Run) };
        self.alloc.reset();
    }
}

/// An expandable scratch space for defining a dynamic bundle. This is similar to [`BundleScratch`],
/// but it uses raw pointers instead of references with lifetimes. This makes it _significantly_
/// harder to use safely, so it should only be used in contexts where it can be easily verified.
#[derive(Default)]
struct OwnedBundleScratch {
    component_ids: Vec<ComponentId>,
    component_ptrs: Vec<NonNull<u8>>,
}

impl OwnedBundleScratch {
    /// Pushes the `ptr` component onto this storage with the given `id` [`ComponentId`].
    ///
    /// # Safety
    /// The `id` [`ComponentId`] must match the component `ptr` for whatever [`World`] this scratch will
    /// be written to. `ptr` must contain valid uniquely-owned data that matches the type of component referenced
    /// in `id`.
    pub unsafe fn push_ptr(&mut self, id: ComponentId, ptr: NonNull<u8>) {
        self.component_ids.push(id);
        self.component_ptrs.push(ptr);
    }

    /// Writes the scratch components to the given entity in the given world.
    ///
    /// # Safety
    /// All [`ComponentId`] values in this instance must come from `world`.
    pub unsafe fn write(
        &mut self,
        entity: &mut EntityWorldMut,
        relationship_hook_insert_mode: RelationshipHookMode,
    ) {
        // SAFETY:
        // - All `component_ids` are from the same world as `entity`
        // - All `component_data_ptrs` are valid types represented by `component_ids`
        unsafe {
            entity.insert_by_ids_internal(
                &self.component_ids,
                self.component_ptrs.drain(..).map(|ptr| OwningPtr::new(ptr)),
                relationship_hook_insert_mode,
            );
        }
        self.component_ids.clear();
    }
}
