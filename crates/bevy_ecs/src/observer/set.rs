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
    type ReadOnlyItem<'trigger>;

    /// Safely casts the pointer to the output type, or a variant of it.
    /// Returns `None` if the event type does not match.
    unsafe fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Item<'trigger>>;

    /// Initialize the components required by the event set.
    fn init_components(world: &mut World, ids: impl FnMut(ComponentId));

    /// Shrink the item to a shorter lifetime.
    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short>;

    /// Shrink the item to a shorter lifetime, read-only variant.
    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short>;

    /// Shrink the item to a shorter lifetime, as a type-erased pointer.
    fn shrink_ptr<'long: 'short, 'short>(item: &'short Self::Item<'long>) -> Ptr<'short>;
}

// SAFETY: Forwards its implementation to `(A,)`, which is itself safe.
unsafe impl<A: Event> EventSet for A {
    type Item<'trigger> = &'trigger mut A;
    type ReadOnlyItem<'trigger> = &'trigger A;

    unsafe fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Item<'trigger>> {
        <(A,) as EventSet>::checked_cast(world, observer_trigger, ptr)
    }

    fn init_components(world: &mut World, ids: impl FnMut(ComponentId)) {
        <(A,) as EventSet>::init_components(world, ids);
    }

    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
        <(A,) as EventSet>::shrink(item)
    }

    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short> {
        <(A,) as EventSet>::shrink_readonly(item)
    }

    fn shrink_ptr<'long: 'short, 'short>(item: &'short Self::Item<'long>) -> Ptr<'short> {
        <(A,) as EventSet>::shrink_ptr(item)
    }
}

// SAFETY: All event types have a component id registered in `init_components`,
// and `checked_cast` checks that the component id matches the event type.
unsafe impl<A: Event> EventSet for (A,) {
    type Item<'trigger> = &'trigger mut A;
    type ReadOnlyItem<'trigger> = &'trigger A;

    unsafe fn checked_cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Item<'trigger>> {
        // SAFETY: Caller must ensure that the component id matches the event type
        Some(unsafe { ptr.deref_mut() })
    }

    fn init_components(world: &mut World, mut ids: impl FnMut(ComponentId)) {
        let a_id = world.init_component::<A>();
        ids(a_id);
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

/// The output type of an observer that observes two different event types.
pub enum Or2<A, B> {
    /// The first event type.
    A(A),
    /// The second event type.
    B(B),
}

// SAFETY: All event types have a component id registered in `init_components`,
// and `checked_cast` checks that the component id matches one of the event types before casting.
unsafe impl<A: Event, B: Event> EventSet for (A, B) {
    type Item<'trigger> = Or2<&'trigger mut A, &'trigger mut B>;
    type ReadOnlyItem<'trigger> = Or2<&'trigger A, &'trigger B>;

    unsafe fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Item<'trigger>> {
        let a_id = world.component_id::<A>()?;
        let b_id = world.component_id::<B>()?;

        if a_id == observer_trigger.event_type {
            // SAFETY: We just checked that the component id matches the event type
            let a = unsafe { ptr.deref_mut() };
            Some(Or2::A(a))
        } else if b_id == observer_trigger.event_type {
            // SAFETY: We just checked that the component id matches the event type
            let b = unsafe { ptr.deref_mut() };
            Some(Or2::B(b))
        } else {
            None
        }
    }

    fn init_components(world: &mut World, mut ids: impl FnMut(ComponentId)) {
        let a_id = world.init_component::<A>();
        let b_id = world.init_component::<B>();
        ids(a_id);
        ids(b_id);
    }

    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
        match item {
            Or2::A(a) => Or2::A(a),
            Or2::B(b) => Or2::B(b),
        }
    }

    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short> {
        match item {
            Or2::A(a) => Or2::A(a),
            Or2::B(b) => Or2::B(b),
        }
    }

    fn shrink_ptr<'long: 'short, 'short>(item: &'short Self::Item<'long>) -> Ptr<'short> {
        match item {
            Or2::A(a) => Ptr::from(&**a),
            Or2::B(b) => Ptr::from(&**b),
        }
    }
}

/// The output type of an observer that observes three different event types.
pub enum Or3<A, B, C> {
    /// The first event type.
    A(A),
    /// The second event type.
    B(B),
    /// The third event type.
    C(C),
}

// SAFETY: All event types have a component id registered in `init_components`,
// and `checked_cast` checks that the component id matches one of the event types before casting.
unsafe impl<A: Event, B: Event, C: Event> EventSet for (A, B, C) {
    type Item<'trigger> = Or3<&'trigger mut A, &'trigger mut B, &'trigger mut C>;
    type ReadOnlyItem<'trigger> = Or3<&'trigger A, &'trigger B, &'trigger C>;

    unsafe fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Item<'trigger>> {
        let a_id = world.component_id::<A>()?;
        let b_id = world.component_id::<B>()?;
        let c_id = world.component_id::<C>()?;

        if a_id == observer_trigger.event_type {
            // SAFETY: We just checked that the component id matches the event type
            let a = unsafe { ptr.deref_mut() };
            Some(Or3::A(a))
        } else if b_id == observer_trigger.event_type {
            // SAFETY: We just checked that the component id matches the event type
            let b = unsafe { ptr.deref_mut() };
            Some(Or3::B(b))
        } else if c_id == observer_trigger.event_type {
            // SAFETY: We just checked that the component id matches the event type
            let c = unsafe { ptr.deref_mut() };
            Some(Or3::C(c))
        } else {
            None
        }
    }

    fn init_components(world: &mut World, mut ids: impl FnMut(ComponentId)) {
        let a_id = world.init_component::<A>();
        let b_id = world.init_component::<B>();
        let c_id = world.init_component::<C>();
        ids(a_id);
        ids(b_id);
        ids(c_id);
    }

    fn shrink<'long: 'short, 'short>(item: &'short mut Self::Item<'long>) -> Self::Item<'short> {
        match item {
            Or3::A(a) => Or3::A(a),
            Or3::B(b) => Or3::B(b),
            Or3::C(c) => Or3::C(c),
        }
    }

    fn shrink_readonly<'long: 'short, 'short>(
        item: &'short Self::Item<'long>,
    ) -> Self::ReadOnlyItem<'short> {
        match item {
            Or3::A(a) => Or3::A(a),
            Or3::B(b) => Or3::B(b),
            Or3::C(c) => Or3::C(c),
        }
    }

    fn shrink_ptr<'long: 'short, 'short>(item: &'short Self::Item<'long>) -> Ptr<'short> {
        match item {
            Or3::A(a) => Ptr::from(&**a),
            Or3::B(b) => Ptr::from(&**b),
            Or3::C(c) => Ptr::from(&**c),
        }
    }
}
