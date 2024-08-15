use bevy_ptr::{Ptr, PtrMut};

use crate::component::ComponentId;
use crate::event::Event;
use crate::observer::ObserverTrigger;
use crate::world::World;

/// A set of [`Event`]s that can trigger an observer.
///
/// The provided implementations of this trait are:
///
/// - All [`Event`]s.
/// - Any tuple of [`Event`]s, up to 15 types. These can be nested.
/// - [`DynamicEvent`], which matches any [`Event`]s dynamically added to the observer with [`Observer::with_event`] and does not reify the event data.
/// - [`SemiDynamicEvent`], which will first try to match a statically-known set of [`Event`]s and reify the event data,
///   and if no match is found, it will fall back to functioning as a [`DynamicEvent`].
///
/// # Example
///
/// TODO
///
/// # Safety
///
/// Implementor must ensure that:
/// - [`EventSet::init_components`] must register a [`ComponentId`] for each [`Event`] type in the set.
/// - [`EventSet::matches`] must return `true` if the triggered [`Event`]'s [`ComponentId`] matches a type in the set,
///   or unambiguously always returns `true` or `false`.
///
/// [`Observer::with_event`]: crate::observer::Observer::with_event
pub unsafe trait EventSet: 'static {
    /// The item returned by this [`EventSet`] that will be passed to the observer system function.
    /// Most of the time this will be a mutable reference to an [`Event`] type, a tuple of mutable references, or a [`PtrMut`].
    type Item<'trigger>;
    /// The read-only variant of the [`Item`](EventSet::Item).
    type ReadOnlyItem<'trigger>: Copy;

    /// Safely casts a pointer to the [`Item`](EventSet::Item) type by checking prior
    /// whether the triggered [`Event`]'s [`ComponentId`] [`matches`](EventSet::matches) a type in this event set.
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

    /// Casts a pointer to the [`Item`](EventSet::Item) type
    /// without checking if the [`Event`] type matches this event set.
    ///
    /// # Safety
    ///
    /// Caller must ensure that the [`Event`]'s [`ComponentId`] [`matches`](EventSet::matches)
    /// this event set before calling this function.
    unsafe fn unchecked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Result<Self::Item<'trigger>, PtrMut<'trigger>>;

    /// Checks if the [`Event`] type matches the observer trigger.
    ///
    /// # Safety
    ///
    /// Implementors must ensure that this function returns `true`
    /// if the triggered [`Event`]'s [`ComponentId`] matches a type in the set,
    /// or unambiguously always returns `true` or `false`.
    fn matches(world: &World, observer_trigger: &ObserverTrigger) -> bool;

    /// Initialize the components required by this event set.
    fn init_components(world: &mut World, ids: impl FnMut(ComponentId));

    /// Shrink the [`Item`](EventSet::Item) to a shorter lifetime.
    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short>;

    /// Shrink the [`Item`](EventSet::Item) to a shorter lifetime [`ReadOnlyItem`](EventSet::ReadOnlyItem).
    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short>;
}

/// An [`EventSet`] that matches a statically pre-defined set of event types.
///
/// This trait is required in order to prevent a footgun where a user might accidentally specify an `EventSet` similar to
/// `(DynamicEvent, EventA, EventB)`, which would always match `DynamicEvent` and never `EventA` or `EventB`.
/// Therefore, we prevent the introduction of `DynamicEvent` in a static `EventSet`,
/// most notably any `EventSet` tuple made up of normal [`Event`] types.
///
/// If you need to support both dynamic and static event types in a single observer,
/// you can use [`SemiDynamicEvent`] instead.
///
/// # Safety
///
/// Implementors must ensure that [`matches`](EventSet::matches)
/// returns `true` if and only if the event component id matches the event type,
/// and DOES NOT match any other event type.
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

/// An [`EventSet`] that matches any event type and performs no casting. Instead, it returns the pointer as is.
/// This is useful for observers that do not need to access the event data, or need to do so dynamically.
///
/// # Example
///
/// TODO
pub struct DynamicEvent;

// SAFETY: Performs no unsafe operations, returns the pointer as is.
unsafe impl EventSet for DynamicEvent {
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
        // We're treating this as a catch-all event set, so it always matches.
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

/// An [`EventSet`] that either matches a statically pre-defined set of event types and casts the pointer to the event type,
/// or returns the pointer as-is if the event type was not matched.
/// Basically, it allows you to mix static and dynamic event types in a single observer.
///
/// `SemiDynamicEvent` accepts two type parameters:
///
/// - **Static**
///   The static event set that will be matched and casted.
///   Generally, this should be a tuple of static event types, like `(FooEvent, BarEvent)`.
///   Must implement [`StaticEventSet`] trait, which means no [`DynamicEvent`] or [`SemiDynamicEvent`] nesting.
///
/// # Example
///
/// TODO
pub struct SemiDynamicEvent<Static: StaticEventSet>(std::marker::PhantomData<Static>);

// SAFETY: No unsafe operations are performed. The checked cast variant is used for the static event type.
unsafe impl<Static: StaticEventSet> EventSet for SemiDynamicEvent<Static> {
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

// SAFETY: The inner event set is a static event set.
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

// We can't use `all_tuples` here because it doesn't support the extra `OrX` parameter required for each tuple impl.
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
