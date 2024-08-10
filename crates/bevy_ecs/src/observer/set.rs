use bevy_ptr::{Ptr, PtrMut};

use crate::component::ComponentId;
use crate::event::Event;
use crate::observer::ObserverTrigger;
use crate::world::World;

/// A set of events that can trigger an observer.
///
/// # Safety
///
/// Implementor must ensure that:
/// - [`EventSet::init_components`] must register a component id for each event type in the set.
/// - [`EventSet::matches`] must return `true` if and only if the event type is in the set.
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
}

// SAFETY: Forwards to the inner event type, and inherits its safety properties.
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
        // and `unchecked_cast` calls `matches` before casting to one of the inner event sets.
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
impl_event_set!(
    Or9,
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i)
);
impl_event_set!(
    Or10,
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j)
);
impl_event_set!(
    Or11,
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j),
    (K, k)
);
impl_event_set!(
    Or12,
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j),
    (K, k),
    (L, l)
);
impl_event_set!(
    Or13,
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j),
    (K, k),
    (L, l),
    (M, m)
);
impl_event_set!(
    Or14,
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j),
    (K, k),
    (L, l),
    (M, m),
    (N, n)
);
impl_event_set!(
    Or15,
    (A, a),
    (B, b),
    (C, c),
    (D, d),
    (E, e),
    (F, f),
    (G, g),
    (H, h),
    (I, i),
    (J, j),
    (K, k),
    (L, l),
    (M, m),
    (N, n),
    (O, o)
);

/// A wrapper around an [`EventSet`] that foregoes safety checks and casting, and passes the pointer as is.
/// This is useful for observers that do not need to access the event data, or need to do so dynamically.
pub struct UntypedEvent<E = ()>(std::marker::PhantomData<E>);

/// An [`EventSet`] that matches the specified event type(s), but does not cast the pointer.
// SAFETY: Performs no unsafe operations, returns the pointer as is.
unsafe impl<E: EventSet> EventSet for UntypedEvent<E> {
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
        true
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
}

/// An [`EventSet`] that matches any event type, but does not cast the pointer.
// SAFETY: Performs no unsafe operations, returns the pointer as is.
unsafe impl EventSet for UntypedEvent<()> {
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
}
