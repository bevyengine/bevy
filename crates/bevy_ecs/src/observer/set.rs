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
/// - [`EventSet::matches`] must return `true` if the event type is in the set, or if the set matches any event type.
///
pub unsafe trait EventSet: 'static {
    /// The output type that will be passed to the observer.
    type Item<'trigger>;
    /// The read-only variant of the output type.
    type ReadOnlyItem<'trigger>: Copy;

    /// Safely casts a pointer to the output type, checking if the event type matches this event set.
    fn cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>> {
        if Self::matches(world, observer_trigger) {
            // SAFETY: We have checked that the event component id matches the event type
            unsafe { Self::unchecked_cast(world, observer_trigger, ptr) }
        } else {
            Err(ptr)
        }
    }

    /// Casts a pointer to the output type.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the event component id [`matches`](EventSet::matches) this event set before calling this function.
    unsafe fn unchecked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>>;

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

/// An [`EventSet`] that matches a statically pre-defined set of event types.
///
/// # Safety
///
/// Implementors must ensure that [`matches`](EventSet::matches)
/// returns `true` if and only if the event component id matches the event type,
/// AND does not match any other event type.
pub unsafe trait StaticEventSet: EventSet {}

// SAFETY: The event type has a component id registered in `init_components`,
// and `matches` checks that the event component id matches the event type.
unsafe impl<E: Event> EventSet for E {
    type Item<'trigger> = &'trigger mut E;
    type ReadOnlyItem<'trigger> = &'trigger E;

    unsafe fn unchecked_cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>> {
        // SAFETY: Caller must ensure that the component id matches the event type
        Ok(unsafe { ptr.deref_mut() })
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

// SAFETY: The event type is a statically known type.
unsafe impl<E: Event> StaticEventSet for E {}

/// An [`EventSet`] that matches any event type, but does not cast the pointer. Instead, it returns the pointer as is.
/// This is useful for observers that do not need to access the event data, or need to do so dynamically.
pub struct DynamicEvent<Register = ()>(std::marker::PhantomData<Register>);

// SAFETY: Performs no unsafe operations, returns the pointer as is.
unsafe impl<Register: StaticEventSet> EventSet for DynamicEvent<Register> {
    type Item<'trigger> = PtrMut<'trigger>;
    type ReadOnlyItem<'trigger> = Ptr<'trigger>;

    fn cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>> {
        Ok(ptr)
    }

    unsafe fn unchecked_cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>> {
        Ok(ptr)
    }

    fn matches(_world: &World, _observer_trigger: &ObserverTrigger) -> bool {
        true
    }

    fn init_components(world: &mut World, ids: impl FnMut(ComponentId)) {
        Register::init_components(world, ids);
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

// SAFETY: Performs no unsafe operations, returns the pointer as is.
unsafe impl EventSet for DynamicEvent<()> {
    type Item<'trigger> = PtrMut<'trigger>;
    type ReadOnlyItem<'trigger> = Ptr<'trigger>;

    fn cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>> {
        Ok(ptr)
    }

    unsafe fn unchecked_cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>> {
        Ok(ptr)
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

/// An [`EventSet`] that matches a statically pre-defined set of event types and casts the pointer to the event type,
/// or returns the pointer as is if the event type does not match.
pub struct SemiDynamicEvent<Static: StaticEventSet, Register = ()>(
    std::marker::PhantomData<(Static, Register)>,
);

// SAFETY: No unsafe operations are performed. The checked cast variant is used for the static event type.
unsafe impl<Static: StaticEventSet, Register> EventSet for SemiDynamicEvent<Static, Register>
where
    DynamicEvent<Register>: EventSet,
{
    type Item<'trigger> = Result<Static::Item<'trigger>, PtrMut<'trigger>>;
    type ReadOnlyItem<'trigger> = Result<Static::ReadOnlyItem<'trigger>, Ptr<'trigger>>;

    unsafe fn unchecked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>> {
        match Static::cast(world, observer_trigger, ptr) {
            Ok(item) => Ok(Ok(item)),
            Err(ptr) => Ok(Err(ptr)),
        }
    }

    fn matches(_world: &World, _observer_trigger: &ObserverTrigger) -> bool {
        true
    }

    fn init_components(world: &mut World, mut ids: impl FnMut(ComponentId)) {
        Static::init_components(world, &mut ids);
        DynamicEvent::<Register>::init_components(world, ids);
    }

    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
        match item {
            Ok(item) => Ok(Static::shrink(item)),
            Err(ptr) => Err(ptr.reborrow()),
        }
    }

    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short> {
        match item {
            Ok(item) => Ok(Static::shrink_readonly(item)),
            Err(ptr) => Err(ptr.as_ref()),
        }
    }
}

// SAFETY: Forwards to the inner event type, and inherits its safety properties.
unsafe impl<A: StaticEventSet> EventSet for (A,) {
    type Item<'trigger> = A::Item<'trigger>;
    type ReadOnlyItem<'trigger> = A::ReadOnlyItem<'trigger>;

    fn cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>> {
        A::cast(world, observer_trigger, ptr)
    }

    unsafe fn unchecked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>> {
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

unsafe impl<A: StaticEventSet> StaticEventSet for (A,) {}

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
        unsafe impl<$($P: StaticEventSet),*> EventSet for ($($P,)*) {
            type Item<'trigger> = $Or<$($P::Item<'trigger>),*>;
            type ReadOnlyItem<'trigger> = $Or<$($P::ReadOnlyItem<'trigger>),*>;

            fn cast<'trigger>(
                world: &World,
                observer_trigger: &ObserverTrigger,
                ptr: PtrMut<'trigger>,
            ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>> {
                // SAFETY: Each inner event set is checked in order for a match and then casted.
                unsafe { Self::unchecked_cast(world, observer_trigger, ptr) }
            }

            unsafe fn unchecked_cast<'trigger>(
                world: &World,
                observer_trigger: &ObserverTrigger,
                ptr: PtrMut<'trigger>,
            ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>> {
                if false {
                    unreachable!();
                }
                $(
                    else if $P::matches(world, observer_trigger) {
                        match $P::unchecked_cast(world, observer_trigger, ptr) {
                            Ok($p) => return Ok($Or::$P($p)),
                            Err(ptr) => return Err(ptr),
                        }
                    }
                )*

                Err(ptr)
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

        // SAFETY: All inner event types are static event sets.
        unsafe impl<$($P: StaticEventSet),*> StaticEventSet for ($($P,)*) {}
    };
}

#[rustfmt::skip] impl_event_set!(Or2, (A, a), (B, b));
#[rustfmt::skip] impl_event_set!(Or3, (A, a), (B, b), (C, c));
#[rustfmt::skip] impl_event_set!(Or4, (A, a), (B, b), (C, c), (D, d));
#[rustfmt::skip] impl_event_set!(Or5, (A, a), (B, b), (C, c), (D, d), (E, e));
#[rustfmt::skip] impl_event_set!(Or6, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f));
#[rustfmt::skip] impl_event_set!(Or7, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f), (G, g));
#[rustfmt::skip] impl_event_set!(Or8, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f), (G, g), (H, h));
#[rustfmt::skip] impl_event_set!(Or9, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f), (G, g), (H, h), (I, i));
#[rustfmt::skip] impl_event_set!(Or10, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f), (G, g), (H, h), (I, i), (J, j));
#[rustfmt::skip] impl_event_set!(Or11, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f), (G, g), (H, h), (I, i), (J, j), (K, k));
#[rustfmt::skip] impl_event_set!(Or12, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f), (G, g), (H, h), (I, i), (J, j), (K, k), (L, l));
#[rustfmt::skip] impl_event_set!(Or13, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f), (G, g), (H, h), (I, i), (J, j), (K, k), (L, l), (M, m));
#[rustfmt::skip] impl_event_set!(Or14, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f), (G, g), (H, h), (I, i), (J, j), (K, k), (L, l), (M, m), (N, n));
#[rustfmt::skip] impl_event_set!(Or15, (A, a), (B, b), (C, c), (D, d), (E, e), (F, f), (G, g), (H, h), (I, i), (J, j), (K, k), (L, l), (M, m), (N, n), (O, o));
