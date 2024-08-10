use bevy_ptr::{Ptr, PtrMut};

use crate::component::ComponentId;
use crate::event::Event;
use crate::observer::ObserverTrigger;
use crate::world::World;

/// A set of events that can trigger an observer.
///
/// # Safety
///
/// Implementor must ensure that [`checked_cast`] and [`init_components`] obey the following:
/// - Each event type must have a component id registered in [`init_components`].
/// - [`checked_cast`] must check that the component id matches the event type in order to safely cast a pointer to the output type.
///
pub unsafe trait EventSet: 'static {
    /// The output type that will be passed to the observer.
    type Item<'trigger>;
    /// The read-only variant of the output type.
    type ReadOnlyItem<'trigger>: Copy;

    /// Casts a pointer to the output type.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the component id [`EventSet::matches`] this event set before calling this function.
    unsafe fn unchecked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Item<'trigger>>;

    /// Checks if the event type matches the observer trigger.
    fn matches(world: &World, observer_trigger: &ObserverTrigger) -> bool;

    /// Initialize the components required by the event set.
    fn init_components(world: &mut World, ids: impl FnMut(ComponentId));

    /// Shrink the item to a shorter lifetime.
    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short>;

    /// Shrink the item to a shorter lifetime, read-only variant.
    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short>;

    /// Shrink the item to a shorter lifetime, as a read-only type-erased pointer.
    ///
    /// # Safety
    ///
    /// Implementor must give a pointer to the innermost data, not a pointer to any container.
    fn shrink_ptr<'long: 'short, 'short>(item: &'short Self::Item<'long>) -> Ptr<'short>;
}

// SAFETY: The event type has a component id registered in `init_components`,
// and `matches` checks that the component id matches the event type.
unsafe impl<E: Event> EventSet for E {
    type Item<'trigger> = &'trigger mut E;
    type ReadOnlyItem<'trigger> = &'trigger E;

    unsafe fn unchecked_cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Item<'trigger>> {
        // SAFETY: Caller must ensure that the component id matches the event type
        Some(unsafe { ptr.deref_mut() })
    }

    fn matches(world: &World, observer_trigger: &ObserverTrigger) -> bool {
        world
            .component_id::<E>()
            .is_some_and(|id| id == observer_trigger.event_type)
    }

    fn init_components(world: &mut World, mut ids: impl FnMut(ComponentId)) {
        let id = world.init_component::<E>();
        ids(id);
    }

    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
        item
    }

    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short> {
        item
    }

    fn shrink_ptr<'long: 'short, 'short>(item: &'short Self::Item<'long>) -> Ptr<'short> {
        Ptr::from(&**item)
    }
}

unsafe impl<A: EventSet> EventSet for (A,) {
    type Item<'trigger> = A::Item<'trigger>;
    type ReadOnlyItem<'trigger> = A::ReadOnlyItem<'trigger>;

    unsafe fn unchecked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Item<'trigger>> {
        A::unchecked_cast(world, observer_trigger, ptr)
    }

    fn matches(world: &World, observer_trigger: &ObserverTrigger) -> bool {
        A::matches(world, observer_trigger)
    }

    fn init_components(world: &mut World, ids: impl FnMut(ComponentId)) {
        A::init_components(world, ids);
    }

    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
        A::shrink(item)
    }

    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short> {
        A::shrink_readonly(item)
    }

    fn shrink_ptr<'long: 'short, 'short>(item: &'short Self::Item<'long>) -> Ptr<'short> {
        A::shrink_ptr(item)
    }
}

macro_rules! impl_event_set {
    ($Or:ident, $(($P:ident, $p:ident)),*) => {
        /// An output type of an observer that observes multiple event types.
        #[derive(Copy, Clone)]
        pub enum $Or<$($P),*> {
            $(
                /// A possible event type.
                $P($P),
            )*
        }

        // SAFETY: All event types have a component id registered in `init_components`,
        // and `cast` calls `matches` before casting the pointer to one of the event types.
        unsafe impl<$($P: EventSet),*> EventSet for ($($P,)*) {
            type Item<'trigger> = $Or<$($P::Item<'trigger>),*>;
            type ReadOnlyItem<'trigger> = $Or<$($P::ReadOnlyItem<'trigger>),*>;

            unsafe fn unchecked_cast<'trigger>(
                world: &World,
                observer_trigger: &ObserverTrigger,
                ptr: PtrMut<'trigger>,
            ) -> Option<Self::Item<'trigger>> {
                if false {
                    unreachable!();
                }
                $(
                    else if $P::matches(world, observer_trigger) {
                        if let Some($p) = $P::unchecked_cast(world, observer_trigger, ptr) {
                            return Some($Or::$P($p));
                        }
                    }
                )*

                None
            }

            fn matches(world: &World, observer_trigger: &ObserverTrigger) -> bool {
                $(
                    $P::matches(world, observer_trigger) ||
                )*
                false
            }

            fn init_components(world: &mut World, mut ids: impl FnMut(ComponentId)) {
                $(
                    $P::init_components(world, &mut ids);
                )*
            }

            fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
                match item {
                    $(
                        $Or::$P($p) => $Or::$P($P::shrink($p)),
                    )*
                }
            }

            fn shrink_readonly<'long: 'short, 'short>(
                item: &'short Self::Item<'long>,
            ) -> Self::ReadOnlyItem<'short> {
                match item {
                    $(
                        $Or::$P($p) => $Or::$P($P::shrink_readonly($p)),
                    )*
                }
            }

            fn shrink_ptr<'long: 'short, 'short>(item: &'short Self::Item<'long>) -> Ptr<'short> {
                match item {
                    $(
                        $Or::$P($p) => $P::shrink_ptr($p),
                    )*
                }
            }
        }
    };
}

impl_event_set!(Or2, (A, a), (B, b));
impl_event_set!(Or3, (A, a), (B, b), (C, c));
impl_event_set!(Or4, (A, a), (B, b), (C, c), (D, d));
impl_event_set!(Or5, (A, a), (B, b), (C, c), (D, d), (E, e));
impl_event_set!(Or6, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f));
impl_event_set!(Or7, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f), (G, g));
impl_event_set!(
    Or8,
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h)
);

/// A wrapper around an [`EventSet`] that foregoes safety checks and casting, and passes the pointer as is.
pub struct Untyped<E>(std::marker::PhantomData<E>);

/// An [`EventSet`] that matches the specified event type(s), but does not cast the pointer.
unsafe impl<E: EventSet> EventSet for Untyped<E> {
    type Item<'trigger> = PtrMut<'trigger>;
    type ReadOnlyItem<'trigger> = Ptr<'trigger>;

    unsafe fn unchecked_cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Item<'trigger>> {
        Some(ptr)
    }

    fn matches(world: &World, observer_trigger: &ObserverTrigger) -> bool {
        E::matches(world, observer_trigger)
    }

    fn init_components(world: &mut World, ids: impl FnMut(ComponentId)) {
        E::init_components(world, ids);
    }

    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
        item.reborrow()
    }

    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short> {
        item.as_ref()
    }

    fn shrink_ptr<'long: 'short, 'short>(item: &'short Self::Item<'long>) -> Ptr<'short> {
        item.as_ref()
    }
}

/// An [`EventSet`] that matches any event type.
unsafe impl EventSet for Untyped<()> {
    type Item<'trigger> = PtrMut<'trigger>;
    type ReadOnlyItem<'trigger> = Ptr<'trigger>;

    unsafe fn unchecked_cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Item<'trigger>> {
        Some(ptr)
    }

    fn matches(_world: &World, _observer_trigger: &ObserverTrigger) -> bool {
        true
    }

    fn init_components(_world: &mut World, _ids: impl FnMut(ComponentId)) {}

    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
        item.reborrow()
    }

    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short> {
        item.as_ref()
    }

    fn shrink_ptr<'long: 'short, 'short>(item: &'short Self::Item<'long>) -> Ptr<'short> {
        item.as_ref()
    }
}
