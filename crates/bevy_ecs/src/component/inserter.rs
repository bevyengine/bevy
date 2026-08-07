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
        let c = (self.constructor)();
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
            // - checked in constructor that the component is a ZST, so the dangling pointer is valid
            unsafe { entity.insert_by_id(component_id, p) };
            Ok(())
        }
    }
}

mod backing_ptr {
    use core::alloc::Layout;
    use core::ptr::NonNull;

    use alloc::alloc::dealloc;
    use alloc::boxed::Box;

    use bevy_ptr::OwningPtr;

    /// Holds memory for a single [`OwningPtr`]
    pub(super) struct BackingPtr {
        layout: Layout,
        ptr: NonNull<u8>,
    }

    impl BackingPtr {
        pub(super) fn new<C: Send + 'static>(c: Box<C>) -> Self {
            Self {
                layout: Layout::new::<C>(),
                ptr: NonNull::new(Box::into_raw(c)).unwrap().cast(),
            }
        }

        /// # Safety
        /// Only one [`OwningPtr`] may be crated from any given [`BackingPtr`]
        pub(super) unsafe fn owning_ptr(&self) -> OwningPtr<'_> {
            // SAFETY: ptr came from a box and the caller guarantees that only one is created
            unsafe { OwningPtr::new(self.ptr) }
        }
    }

    // SAFETY: constructor ensures this is only created for `Send` types
    unsafe impl Send for BackingPtr {}

    impl Drop for BackingPtr {
        fn drop(&mut self) {
            // SAFETY: ptr came from a box and it's not used after this
            unsafe { dealloc(self.ptr.as_ptr(), self.layout) };
        }
    }
}
