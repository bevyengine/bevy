// TODO: This probably doesn't belong in bevy_ui_widgets, but I am not sure where it should go.
// It is certainly a useful thing to have.
#![expect(unsafe_code, reason = "Unsafe code is used to improve performance.")]

use core::marker::PhantomData;

use bevy_ecs::{
    bundle::{Bundle, DynamicBundle},
    event::TargetEvent,
    system::IntoObserverSystem,
};

/// Helper struct that adds an observer when inserted as a [`Bundle`].
pub struct AddObserver<E: TargetEvent, B: Bundle, M, I: IntoObserverSystem<E, B, M>> {
    observer: I,
    marker: PhantomData<(E, B, M)>,
}

// SAFETY: Empty method bodies.
unsafe impl<
        E: TargetEvent,
        B: Bundle,
        M: Send + Sync + 'static,
        I: IntoObserverSystem<E, B, M> + Send + Sync,
    > Bundle for AddObserver<E, B, M, I>
{
    #[inline]
    fn component_ids(
        _components: &mut bevy_ecs::component::ComponentsRegistrator,
        _ids: &mut impl FnMut(bevy_ecs::component::ComponentId),
    ) {
        // SAFETY: Empty function body
    }

    #[inline]
    fn get_component_ids(
        _components: &bevy_ecs::component::Components,
        _ids: &mut impl FnMut(Option<bevy_ecs::component::ComponentId>),
    ) {
        // SAFETY: Empty function body
    }
}

impl<E: TargetEvent, B: Bundle, M, I: IntoObserverSystem<E, B, M>> DynamicBundle
    for AddObserver<E, B, M, I>
{
    type Effect = Self;

    #[inline]
    unsafe fn get_components(
        _ptr: bevy_ecs::ptr::MovingPtr<'_, Self>,
        _func: &mut impl FnMut(bevy_ecs::component::StorageType, bevy_ecs::ptr::OwningPtr<'_>),
    ) {
        // SAFETY: Empty function body
    }

    #[inline]
    unsafe fn apply_effect(
        ptr: bevy_ecs::ptr::MovingPtr<'_, core::mem::MaybeUninit<Self>>,
        entity: &mut bevy_ecs::world::EntityWorldMut,
    ) {
        // SAFETY: `get_components` does nothing, value was not moved.
        let add_observer = unsafe { ptr.assume_init() };
        let add_observer = add_observer.read();
        entity.observe(add_observer.observer);
    }
}

/// Adds an observer as a bundle effect.
pub fn observe<E: TargetEvent, B: Bundle, M, I: IntoObserverSystem<E, B, M>>(
    observer: I,
) -> AddObserver<E, B, M, I> {
    AddObserver {
        observer,
        marker: PhantomData,
    }
}
