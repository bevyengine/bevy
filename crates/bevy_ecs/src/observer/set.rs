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
    fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Out<'trigger>>;

    /// Initialize the components required by the event set.
    fn init_components(world: &mut World, ids: impl FnMut(ComponentId));
}

unsafe impl<A: Event> EventSet for A {
    type Out<'trigger> = &'trigger mut A;
    type OutReadonly<'trigger> = &'trigger A;

    fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Out<'trigger>> {
        <(A,) as EventSet>::checked_cast(world, observer_trigger, ptr)
    }

    fn init_components(world: &mut World, ids: impl FnMut(ComponentId)) {
        <(A,) as EventSet>::init_components(world, ids)
    }
}

unsafe impl<A: Event> EventSet for (A,) {
    type Out<'trigger> = &'trigger mut A;
    type OutReadonly<'trigger> = &'trigger A;

    fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Out<'trigger>> {
        let Some(a_id) = world.component_id::<A>() else {
            return None;
        };

        if a_id == observer_trigger.event_type {
            // SAFETY: We just checked that the component id matches the event type
            let a = unsafe { ptr.deref_mut() };
            Some(a)
        } else {
            None
        }
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

impl<'a, A, B> From<&'a Or2<&'a mut A, &'a mut B>> for Or2<&'a A, &'a B> {
    fn from(or: &'a Or2<&'a mut A, &'a mut B>) -> Self {
        match or {
            Or2::A(a) => Or2::A(a),
            Or2::B(b) => Or2::B(b),
        }
    }
}

unsafe impl<A: Event, B: Event> EventSet for (A, B) {
    type Out<'trigger> = Or2<&'trigger mut A, &'trigger mut B>;
    type OutReadonly<'trigger> = Or2<&'trigger A, &'trigger B>;

    fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Out<'trigger>> {
        let Some(a_id) = world.component_id::<A>() else {
            return None;
        };
        let Some(b_id) = world.component_id::<B>() else {
            return None;
        };

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

impl<'a, A, B, C> From<&'a Or3<&'a mut A, &'a mut B, &'a mut C>> for Or3<&'a A, &'a B, &'a C> {
    fn from(or: &'a Or3<&'a mut A, &'a mut B, &'a mut C>) -> Self {
        match or {
            Or3::A(a) => Or3::A(a),
            Or3::B(b) => Or3::B(b),
            Or3::C(c) => Or3::C(c),
        }
    }
}

unsafe impl<A: Event, B: Event, C: Event> EventSet for (A, B, C) {
    type Out<'trigger> = Or3<&'trigger mut A, &'trigger mut B, &'trigger mut C>;
    type OutReadonly<'trigger> = Or3<&'trigger A, &'trigger B, &'trigger C>;

    fn checked_cast<'trigger>(
        world: &World,
        observer_trigger: &ObserverTrigger,
        ptr: PtrMut<'trigger>,
    ) -> Option<Self::Out<'trigger>> {
        let Some(a_id) = world.component_id::<A>() else {
            return None;
        };
        let Some(b_id) = world.component_id::<B>() else {
            return None;
        };
        let Some(c_id) = world.component_id::<C>() else {
            return None;
        };

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
