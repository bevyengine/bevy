use bevy_ptr::PtrMut;

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
    type Out<'trigger>;
    /// The read-only variant of the output type.
    type OutReadonly<'trigger>;

    /// Safely casts the pointer to the output type, or a variant of it.
    /// Returns `None` if the event type does not match.
    unsafe fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Out<'trigger>>;

    /// Initialize the components required by the event set.
    fn init_components(world: &mut World, ids: impl FnMut(ComponentId));
}

// SAFETY: Forwards its implementation to `(A,)`, which is itself safe.
unsafe impl<A: Event> EventSet for A {
    type Out<'trigger> = &'trigger mut A;
    type OutReadonly<'trigger> = &'trigger A;

    unsafe fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Out<'trigger>> {
        <(A,) as EventSet>::checked_cast(world, observer_trigger, ptr)
    }

    fn init_components(world: &mut World, ids: impl FnMut(ComponentId)) {
        <(A,) as EventSet>::init_components(world, ids);
    }
}

// SAFETY: All event types have a component id registered in `init_components`,
// and `checked_cast` checks that the component id matches the event type.
unsafe impl<A: Event> EventSet for (A,) {
    type Out<'trigger> = &'trigger mut A;
    type OutReadonly<'trigger> = &'trigger A;

    unsafe fn checked_cast<'trigger>(
        _world: &World,
        _observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Out<'trigger>> {
        // SAFETY: Caller must ensure that the component id matches the event type
        Some(unsafe { ptr.deref_mut() })
    }

    fn init_components(world: &mut World, mut ids: impl FnMut(ComponentId)) {
        let a_id = world.init_component::<A>();
        ids(a_id);
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
    type Out<'trigger> = Or2<&'trigger mut A, &'trigger mut B>;
    type OutReadonly<'trigger> = Or2<&'trigger A, &'trigger B>;

    unsafe fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Out<'trigger>> {
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
    type Out<'trigger> = Or3<&'trigger mut A, &'trigger mut B, &'trigger mut C>;
    type OutReadonly<'trigger> = Or3<&'trigger A, &'trigger B, &'trigger C>;

    unsafe fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Out<'trigger>> {
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
}
