use crate::{
    component::{Component, ComponentId, Components, ComponentsRegistrator},
    relationship::RelationshipHookMode,
    world::EntityWorldMut,
};
use alloc::vec::Vec;
use bevy_ptr::OwningPtr;
use bumpalo::Bump;
use core::{alloc::Layout, ptr::NonNull};

/// Enables pushing components to internal scratch space (uses a bump allocator), which can then be
/// written as a dynamic bundle. The contents are cleared after each write and the allocated scratch
/// space is reused across writes.
///
/// Also see [`BundleWriter`].
#[derive(Default)]
pub struct BundleScratch {
    // Correctness: this should never be made public or mismatched component ids could be inserted
    component_ids: Vec<ComponentId>,
    // Correctness: this should never be made public or arbitrary non-components could be inserted
    component_ptrs: Vec<NonNull<u8>>,
    // Safety: this cannot be exposed, otherwise `alloc.reset()` could be called in arbitrary places,
    // which could invalidate the data stored in `component_ptrs`.
    alloc: Bump,
}

impl BundleScratch {
    /// Creates a new [`BundleWriter`] using this scratch space. For safety / correctness, this will
    /// clear any existing components.
    ///
    /// Note that for performance reasons this will _not_ clear the internal allocator. To avoid leaking,
    /// make sure every component pushed to the [`BundleWriter`] is followed by either a
    /// [`BundleWriter::write`] or a [`BundleScratch::manual_drop`].
    #[inline]
    pub fn writer<'a>(&'a mut self) -> BundleWriter<'a> {
        // This is necessary to ensure safety / correctness is maintained in the context of catch_unwind
        // or a skipped `write`
        self.component_ids.clear();
        self.component_ptrs.clear();
        BundleWriter(self)
    }

    /// Returns true if there are currently no components stored in the scratch space.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.component_ids.is_empty()
    }

    /// This will drop all components currently stored in the scratch space. This is generally used to
    /// ensure drops occur in error scenarios.
    ///
    /// # Safety
    /// `components` must be from the same world as the components that were pushed to this writer.
    pub unsafe fn manual_drop(&mut self, components: &Components) {
        for (id, ptr) in self
            .component_ids
            .drain(..)
            .zip(self.component_ptrs.drain(..))
        {
            if let Some(info) = components.get_info(id)
                && let Some(drop) = info.drop()
            {
                // SAFETY: ptr is a valid component that matches the given component id
                unsafe {
                    let ptr = OwningPtr::new(ptr);
                    (drop)(ptr);
                }
            }
        }
        self.alloc.reset();
    }
}

/// Enables pushing components to the internal [`BundleScratch`], which can then be
/// written as a dynamic bundle.
///
/// Components pushed to this writer should either be followed by a [`BundleWriter::write`] or a
/// [`BundleScratch::manual_drop`] to avoid leaking.
pub struct BundleWriter<'a>(&'a mut BundleScratch);

// SAFETY: The `NonNull`s in component_ptrs are always a `Component`, which is Send
unsafe impl Send for BundleScratch where Bump: Send {}

impl<'a> BundleWriter<'a> {
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
        OwningPtr::make(component, |ptr| {
            // SAFETY: ptr points to a C component value which matches the `id` looked up above.
            // Layout matches C.
            self.push_component_by_id(id, ptr, Layout::new::<C>());
        });
    }

    /// Pushes the given component ptr to the back of the current bundle scratch space.
    ///
    /// # Safety
    ///
    /// `components` must be from the same world that all previous [`Self::push_component`] calls were called with,
    /// and the _next_ [`Self::write`] call. `component` must point to a [`Component`] value that matches `id`.
    /// `layout` must correspond to the layout of the [`Component`] type.
    pub unsafe fn push_component_by_id(
        &mut self,
        id: ComponentId,
        component: OwningPtr<'_>,
        layout: Layout,
    ) {
        let ptr = self.0.alloc.alloc_layout(layout);
        core::ptr::copy(component.as_ptr(), ptr.as_ptr(), layout.size());
        self.0.component_ids.push(id);
        self.0.component_ptrs.push(ptr);
    }

    /// Writes the current contents of the bundle to the given `entity` and clears the scratch space.
    ///
    /// # Safety
    ///
    /// `entity` must be from the same world that all [`Self::push_component`] calls since the last
    /// [`Self::write`] were called with.
    pub unsafe fn write(self, entity: &mut EntityWorldMut) {
        // SAFETY:
        // - All `component_ids` are from the same world as `entity`
        // - All `component_data_ptrs` are valid types represented by `component_ids`
        unsafe {
            entity.insert_by_ids_internal(
                &self.0.component_ids,
                self.0
                    .component_ptrs
                    .drain(..)
                    .map(|ptr| OwningPtr::new(ptr)),
                RelationshipHookMode::Run,
            );
        }
        self.0.component_ids.clear();
        self.0.alloc.reset();
    }

    /// Returns true if there are currently no components.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.component_ids.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use crate::{bundle::BundleScratch, component::Component, name::Name, world::World};

    #[test]
    fn write_component() {
        #[derive(Component)]
        struct X;

        let mut world = World::new();
        let mut bundle_scratch = BundleScratch::default();
        let mut bundle_writer = bundle_scratch.writer();
        // SAFETY: the same world is used for every bundle_writer operation
        unsafe {
            let mut components = world.components_registrator();
            bundle_writer.push_component(&mut components, X);
            bundle_writer.push_component(&mut components, Name::new("Hi"));
            let mut entity = world.spawn_empty();
            bundle_writer.write(&mut entity);

            assert_eq!(entity.get::<Name>().unwrap().as_str(), "Hi");
            assert!(entity.contains::<X>());
        }
    }
}
