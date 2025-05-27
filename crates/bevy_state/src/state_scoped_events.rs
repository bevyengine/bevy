use alloc::vec::Vec;
use core::marker::PhantomData;

use bevy_app::{App, SubApp};
use bevy_ecs::{
    event::{Event, EventReader, Events},
    resource::Resource,
    system::Commands,
    world::World,
};
use bevy_platform::collections::HashMap;

use crate::state::{OnExit, StateTransitionEvent, States};

fn clear_event_queue<E: Event>(w: &mut World) {
    if let Some(mut queue) = w.get_resource_mut::<Events<E>>() {
        queue.clear();
    }
}

#[derive(Copy, Clone)]
enum TransitionType {
    OnExit,
    OnEnter,
}

#[derive(Resource)]
struct ClearEventsOnStateTransition<S: States> {
    on_exit: HashMap<S, Vec<fn(&mut World)>>,
    on_enter: HashMap<S, Vec<fn(&mut World)>>,
}

impl<S: States> ClearEventsOnStateTransition<S> {
    fn add_event<E: Event>(&mut self, state: S, transition_type: TransitionType) {
        let map = match transition_type {
            TransitionType::OnExit => &mut self.on_exit,
            TransitionType::OnEnter => &mut self.on_enter,
        };
        map.entry(state).or_default().push(clear_event_queue::<E>);
    }

    fn cleanup(&self, w: &mut World, state: S, transition_type: TransitionType) {
        let map = match transition_type {
            TransitionType::OnExit => &self.on_exit,
            TransitionType::OnEnter => &self.on_enter,
        };
        let Some(fns) = map.get(&state) else {
            return;
        };
        for callback in fns {
            (*callback)(w);
        }
    }
}

impl<S: States> Default for ClearEventsOnStateTransition<S> {
    fn default() -> Self {
        Self {
            on_exit: HashMap::default(),
            on_enter: HashMap::default(),
        }
    }
}

fn clear_events_on_exit_state<S: States>(
    mut c: Commands,
    mut transitions: EventReader<StateTransitionEvent<S>>,
) {
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited {
        return;
    }
    let Some(exited) = transition.exited.clone() else {
        return;
    };

    c.queue(move |w: &mut World| {
        w.resource_scope::<ClearEventsOnStateTransition<S>, ()>(|w, events| {
            events.cleanup(w, exited, TransitionType::OnEnter);
        });
    });
}

fn clear_events_on_enter_state<S: States>(
    mut c: Commands,
    mut transitions: EventReader<StateTransitionEvent<S>>,
) {
    let Some(transition) = transitions.read().last() else {
        return;
    };
    if transition.entered == transition.exited {
        return;
    }
    let Some(entered) = transition.entered.clone() else {
        return;
    };

    c.queue(move |w: &mut World| {
        w.resource_scope::<ClearEventsOnStateTransition<S>, ()>(|w, events| {
            events.cleanup(w, entered, TransitionType::OnEnter);
        });
    });
}

fn clear_event_on_state_transition<E: Event, S: States>(
    app: &mut SubApp,
    _p: PhantomData<E>,
    state: S,
    transition_type: TransitionType,
) {
    if !app.world().contains_resource::<ClearEventsOnStateTransition<S>>() {
        app.init_resource::<ClearEventsOnStateTransition<S>>();
    }
    app.world_mut()
        .resource_mut::<ClearEventsOnStateTransition<S>>()
        .add_event::<E>(state.clone(), transition_type);
    app.add_systems(OnExit(state), match transition_type {
        TransitionType::OnExit => clear_events_on_exit_state::<S>,
        TransitionType::OnEnter => clear_events_on_enter_state::<S>,
    });
}

/// Extension trait for [`App`] adding methods for registering state scoped events.
pub trait StateScopedEventsAppExt {
    /// Clears an [`Event`] when leaving the specified `state`.
    ///
    /// Note that event cleanup is ordered ambiguously relative to [`DespawnOnEnterState`](crate::prelude::DespawnOnEnterState)
    /// and [`DespawnOnExitState`](crate::prelude::DespawnOnExitState) entity
    /// cleanup and the [`OnExit`] schedule for the target state. All of these (state scoped
    /// entities and events cleanup, and `OnExit`) occur within schedule [`StateTransition`](crate::prelude::StateTransition).
    fn clear_event_on_exit_state<E: Event>(&mut self, state: impl States) -> &mut Self;

    /// Clears an [`Event`] when leaving the specified `state`.
    fn clear_event_on_enter_state<E: Event>(&mut self, state: impl States) -> &mut Self;
}

impl StateScopedEventsAppExt for App {
    fn clear_event_on_exit_state<E: Event>(&mut self, state: impl States) -> &mut Self {
        clear_event_on_state_transition(self.main_mut(), PhantomData::<E>, state, TransitionType::OnExit);
        self
    }

    fn clear_event_on_enter_state<E: Event>(&mut self, state: impl States) -> &mut Self {
        clear_event_on_state_transition(self.main_mut(), PhantomData::<E>, state, TransitionType::OnEnter);
        self
    }
}

impl StateScopedEventsAppExt for SubApp {
    fn clear_event_on_exit_state<E: Event>(&mut self, state: impl States) -> &mut Self {
        clear_event_on_state_transition(self, PhantomData::<E>, state, TransitionType::OnExit);
        self
    }

    fn clear_event_on_enter_state<E: Event>(&mut self, state: impl States) -> &mut Self {
        clear_event_on_state_transition(self, PhantomData::<E>, state, TransitionType::OnEnter);
        self
    }
}
