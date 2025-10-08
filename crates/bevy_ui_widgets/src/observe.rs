// TODO: This probably doesn't belong in bevy_ui_widgets, but I am not sure where it should go.
// It is certainly a useful thing to have.
#![expect(unsafe_code, reason = "Unsafe code is used to improve performance.")]

use core::{marker::PhantomData, mem};

use bevy_ecs::{
    bundle::{Bundle, DynamicBundle},
    event::EntityEvent,
    system::IntoObserverSystem,
};

/// Helper struct that adds an observer when inserted as a [`Bundle`].
pub struct AddObserver<E: EntityEvent, B: Bundle + 'static, M, I: IntoObserverSystem<E, B, M>> {
    observer: I,
    marker: PhantomData<(E, B, M)>,
}

// SAFETY: Empty method bodies.
unsafe impl<
        E: EntityEvent,
        B: Bundle + 'static,
        M: Send + Sync + 'static,
        I: IntoObserverSystem<E, B, M> + Send + Sync,
    > Bundle for AddObserver<E, B, M, I>
{
    #[inline]
    fn component_ids(
        _components: &mut bevy_ecs::component::ComponentsRegistrator,
    ) -> impl Iterator<Item = bevy_ecs::component::ComponentId> + use<E, B, M, I> {
        // SAFETY: Empty iterator
        core::iter::empty()
    }

    #[inline]
    fn get_component_ids(
        _components: &bevy_ecs::component::Components,
    ) -> impl Iterator<Item = Option<bevy_ecs::component::ComponentId>> {
        // SAFETY: Empty iterator
        core::iter::empty()
    }
}

impl<E: EntityEvent, B: Bundle + 'static, M, I: IntoObserverSystem<E, B, M>> DynamicBundle
    for AddObserver<E, B, M, I>
{
    type Effect = Self;

    #[inline]
    unsafe fn get_components(
        ptr: bevy_ecs::ptr::MovingPtr<'_, Self>,
        _func: &mut impl FnMut(bevy_ecs::component::StorageType, bevy_ecs::ptr::OwningPtr<'_>),
    ) {
        // SAFETY: We must not drop the pointer here, or it will be uninitialized in `apply_effect`
        // below.
        mem::forget(ptr);
    }

    #[inline]
    unsafe fn apply_effect(
        ptr: bevy_ecs::ptr::MovingPtr<'_, mem::MaybeUninit<Self>>,
        entity: &mut bevy_ecs::world::EntityWorldMut,
    ) {
        // SAFETY: The pointer was not dropped in `get_components`, so the allocation is still
        // initialized.
        let add_observer = unsafe { ptr.assume_init() };
        let add_observer = add_observer.read();
        entity.observe(add_observer.observer);
    }
}

/// Adds an observer as a bundle effect.
pub fn observe<E: EntityEvent, B: Bundle + 'static, M, I: IntoObserverSystem<E, B, M>>(
    observer: I,
) -> AddObserver<E, B, M, I> {
    AddObserver {
        observer,
        marker: PhantomData,
    }
}
