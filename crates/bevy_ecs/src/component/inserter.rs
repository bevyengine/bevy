use core::any::TypeId;

use alloc::boxed::Box;
use alloc::format;

use crate::change_detection::MaybeLocation;
use crate::system::entity_command::EntityCommand;
use crate::world::{EntityWorldMut, World, WorldId};

use super::{Component, ComponentId};

use self::backing_ptr::BackingPtr;

/// Safely insert a type-erased [`Component`].
///
/// # Example
/// ```
/// use bevy_ecs::prelude::*;
/// use bevy_ecs::component::ComponentInserter;
///
/// #[derive(Component)]
/// struct MyComponent(usize);
///
/// fn main() {
///     let mut world = World::new();
///
///     // register the component and create an inserter
///     world.register_component::<MyComponent>();
///     let inserter = ComponentInserter::new(&world, || MyComponent(42));
///
///     let mut schedule = Schedule::default();
///
///     schedule.add_systems(
///         (
///             // add a system that uses the inserter
///             move |mut commands: Commands| {
///                 commands.spawn_empty().queue(inserter.insert());
///             },
///             // see that the component was inserted
///             |query: Query<&MyComponent>| {
///                 assert_eq!(query.single().unwrap().0, 42);
///             },
///         )
///             .chain(),
///     );
///
///     schedule.run(&mut world);
/// }
/// ```
pub struct ComponentInserter {
    component_id: ComponentId,
    world_id: WorldId,
    constructor: Box<dyn Fn() -> BackingPtr + Send + Sync + 'static>,
    created: MaybeLocation,
}

impl core::fmt::Debug for ComponentInserter {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("ComponentInserter")
            .field("component_id", &self.component_id)
            .field("world_id", &self.world_id)
            .field("created", &self.created)
            .finish_non_exhaustive()
    }
}

impl ComponentInserter {
    /// Create a new inserter from a world and a constructor.
    ///
    /// The constructor will be called each time this inserter is used to insert its component.
    ///
    /// # Panics
    /// This function will panic if the component was not registered.
    #[track_caller]
    #[inline]
    pub fn new<C: Component>(
        world: &World,
        constructor: impl Fn() -> C + Send + Sync + 'static,
    ) -> Self {
        let component_id = world
            .components()
            .get_id(TypeId::of::<C>())
            .expect("component should be registered");
        Self {
            component_id,
            world_id: world.id(),
            constructor: Box::new(move || BackingPtr::new(Box::new(constructor()))),
            created: MaybeLocation::caller(),
        }
    }

    /// Create a new inserter that will insert the default value.
    ///
    /// # Panics
    /// This function will panic if the component was not registered.
    #[track_caller]
    #[inline]
    pub fn new_with_default<C: Component + Default>(world: &World) -> Self {
        Self::new(world, C::default)
    }

    /// Returns an [`EntityCommand`] that inserts the component.
    #[track_caller]
    #[inline]
    pub fn insert(&self) -> impl EntityCommand + 'static {
        let mut c = (self.constructor)();
        let component_id = self.component_id;
        let world_id = self.world_id;
        let created = self.created;
        let caller = MaybeLocation::caller();
        move |mut entity: EntityWorldMut<'_>| -> crate::error::Result {
            if world_id != entity.world().id() {
                return Err(format!(
                    "Inserter was created using a different world. Created @ {created}, Called @ {caller}"
                )
                .into());
            }
            // SAFETY: we only call this method once
            let p = unsafe { c.owning_ptr() };
            // SAFETY:
            // - already checked that the ComponentId came from the same world as `entity`
            // - p comes from the constructor which is type-checked in `Self::new`
            unsafe { entity.insert_by_id(component_id, p) };
            Ok(())
        }
    }
}

mod backing_ptr {
    use core::alloc::Layout;
    use core::ptr::NonNull;
    use core::{mem, ptr};

    use alloc::alloc::dealloc;
    use alloc::boxed::Box;

    use bevy_ptr::OwningPtr;

    /// Holds memory for a single [`OwningPtr`]
    pub(super) struct BackingPtr {
        layout: Layout,
        ptr: NonNull<u8>,
        /// # Safety
        /// It must be valid to pass `self.ptr` to `self.drop_fn`
        drop_fn: Option<unsafe fn(NonNull<u8>)>,
    }

    impl BackingPtr {
        pub(super) fn new<C: Send + 'static>(c: Box<C>) -> Self {
            Self {
                layout: Layout::new::<C>(),
                ptr: NonNull::new(Box::into_raw(c)).unwrap().cast(),
                drop_fn: mem::needs_drop::<C>().then_some(|p| {
                    // SAFETY: caller upholds requirements
                    unsafe { ptr::drop_in_place(p.cast::<C>().as_ptr()) };
                }),
            }
        }

        /// # Safety
        /// Only one [`OwningPtr`] may be created from any given [`BackingPtr`]
        pub(super) unsafe fn owning_ptr(&mut self) -> OwningPtr<'_> {
            // the OwningPtr is now responsible for dropping it
            self.drop_fn = None;
            // SAFETY: ptr came from a box and the caller guarantees that only one is created
            unsafe { OwningPtr::new(self.ptr) }
        }
    }

    // SAFETY: constructor ensures this is only created for `Send` types
    unsafe impl Send for BackingPtr {}

    impl Drop for BackingPtr {
        fn drop(&mut self) {
            if let Some(drop_fn) = self.drop_fn {
                // SAFETY:
                // - `Self::new` ensures ptr is valid
                // - `self.ptr` is not used again except for `dealloc`
                unsafe { drop_fn(self.ptr) };
            }
            // SAFETY: ptr came from a box and it's not used after this
            unsafe { dealloc(self.ptr.as_ptr(), self.layout) };
        }
    }
}

#[cfg(test)]
#[allow(
    clippy::allow_attributes,
    clippy::undocumented_unsafe_blocks,
    reason = "tests"
)]
mod tests {
    use core::sync::atomic::{AtomicU8, Ordering};

    use super::*;

    #[test]
    fn backing_ptr_drop() {
        static DROP_FLAG: AtomicU8 = AtomicU8::new(0);

        #[expect(dead_code, reason = "need this not to be a ZST")]
        struct Thing(usize);

        impl Drop for Thing {
            fn drop(&mut self) {
                DROP_FLAG.fetch_add(1, Ordering::SeqCst);
            }
        }

        let p = BackingPtr::new(Box::new(Thing(42)));
        drop(p);
        assert_eq!(DROP_FLAG.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn backing_ptr_no_drop() {
        static DROP_FLAG: AtomicU8 = AtomicU8::new(0);

        #[expect(dead_code, reason = "need this not to be a ZST")]
        struct Thing(usize);

        impl Drop for Thing {
            fn drop(&mut self) {
                DROP_FLAG.fetch_add(1, Ordering::SeqCst);
            }
        }

        let mut p = BackingPtr::new(Box::new(Thing(42)));
        unsafe { p.owning_ptr() };
        drop(p);
        assert_eq!(DROP_FLAG.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn backing_ptr_zst() {
        static DROP_FLAG: AtomicU8 = AtomicU8::new(0);

        struct Thing;

        impl Drop for Thing {
            fn drop(&mut self) {
                DROP_FLAG.fetch_add(1, Ordering::SeqCst);
            }
        }

        let p = BackingPtr::new(Box::new(Thing));
        drop(p);
        assert_eq!(DROP_FLAG.load(Ordering::SeqCst), 1);
    }

    #[test]
    #[should_panic(expected = "expected panic from constructor")]
    fn component_inserter_constructor_panic() {
        #[derive(Component)]
        struct Thing;

        let mut world = World::new();
        let inserter =
            world.component_inserter::<Thing>(|| panic!("expected panic from constructor"));
        let entity = world.spawn_empty();
        inserter.insert().apply(entity);
    }
}
