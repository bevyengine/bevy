//! Helper utilities for adding observers as bundle effects.

#![expect(unsafe_code, reason = "Unsafe code is used to improve performance.")]

use core::{marker::PhantomData, mem};

use crate::{
    bundle::{Bundle, DynamicBundle},
    event::EntityEvent,
    system::IntoObserverSystem,
};

/// Helper struct that adds an observer when inserted as a [`Bundle`].
pub struct AddObserver<E: EntityEvent, B: Bundle, M, I: IntoObserverSystem<E, B, M>> {
    observer: I,
    marker: PhantomData<(E, B, M)>,
}

// SAFETY: Empty method bodies.
unsafe impl<
        E: EntityEvent,
        B: Bundle,
        M: Send + Sync + 'static,
        I: IntoObserverSystem<E, B, M> + Send + Sync,
    > Bundle for AddObserver<E, B, M, I>
{
    #[inline]
    fn component_ids(
        _components: &mut crate::component::ComponentsRegistrator,
    ) -> impl Iterator<Item = crate::component::ComponentId> + use<E, B, M, I> {
        // SAFETY: Empty iterator
        core::iter::empty()
    }

    #[inline]
    fn get_component_ids(
        _components: &crate::component::Components,
    ) -> impl Iterator<Item = Option<crate::component::ComponentId>> {
        // SAFETY: Empty iterator
        core::iter::empty()
    }
}

impl<E: EntityEvent, B: Bundle, M, I: IntoObserverSystem<E, B, M>> DynamicBundle
    for AddObserver<E, B, M, I>
{
    type Effect = Self;

    #[inline]
    unsafe fn get_components(
        ptr: crate::ptr::MovingPtr<'_, Self>,
        _func: &mut impl FnMut(crate::component::StorageType, crate::ptr::OwningPtr<'_>),
    ) {
        // SAFETY: We must not drop the pointer here, or it will be uninitialized in `apply_effect`
        // below.
        mem::forget(ptr);
    }

    #[inline]
    unsafe fn apply_effect(
        ptr: crate::ptr::MovingPtr<'_, mem::MaybeUninit<Self>>,
        entity: &mut crate::world::EntityWorldMut,
    ) {
        // SAFETY: The pointer was not dropped in `get_components`, so the allocation is still
        // initialized.
        let add_observer = unsafe { ptr.assume_init() };
        let add_observer = add_observer.read();
        entity.observe(add_observer.observer);
    }
}

/// Adds an observer as a bundle effect.
pub fn observe<E: EntityEvent, B: Bundle, M, I: IntoObserverSystem<E, B, M>>(
    observer: I,
) -> AddObserver<E, B, M, I> {
    AddObserver {
        observer,
        marker: PhantomData,
    }
}
