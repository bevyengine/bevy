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

#[derive(Resource)]
struct ClearEventsOnExitState<S: States> {
    cleanup_fns: HashMap<S, Vec<fn(&mut World)>>,
}

impl<S: States> ClearEventsOnExitState<S> {
    fn add_event<E: Event>(&mut self, state: S) {
        self.cleanup_fns
            .entry(state)
            .or_default()
            .push(clear_event_queue::<E>);
    }

    fn cleanup(&self, w: &mut World, state: S) {
        let Some(fns) = self.cleanup_fns.get(&state) else {
            return;
        };
        for callback in fns {
            (*callback)(w);
        }
    }
}

impl<S: States> Default for ClearEventsOnExitState<S> {
    fn default() -> Self {
        Self {
            cleanup_fns: HashMap::default(),
        }
    }
}

/// Clears events marked with [`ClearEventsOnExitState<S>`] when their state no
/// longer matches the world state.
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
        w.resource_scope::<ClearEventsOnExitState<S>, ()>(|w, events| {
            events.cleanup(w, exited);
        });
    });
}

fn add_event_cleared_on_exit_state<E: Event, S: States>(
    app: &mut SubApp,
    _p: PhantomData<E>,
    state: S,
) {
    if !app.world().contains_resource::<ClearEventsOnExitState<S>>() {
        app.init_resource::<ClearEventsOnExitState<S>>();
    }
    app.add_event::<E>();
    app.world_mut()
        .resource_mut::<ClearEventsOnExitState<S>>()
        .add_event::<E>(state.clone());
    app.add_systems(OnExit(state), clear_events_on_exit_state::<S>);
}

/// Extension trait for [`App`] adding methods for registering state scoped events.
pub trait StateScopedEventsAppExt {
    /// Adds an [`Event`] that is automatically cleaned up when leaving the specified `state`.
    ///
    /// Note that event cleanup is ordered ambiguously relative to [`DespawnOnEnterState`](crate::prelude::DespawnOnEnterState)
    /// and [`DespawnOnExitState`](crate::prelude::DespawnOnExitState) entity
    /// cleanup and the [`OnExit`] schedule for the target state. All of these (state scoped
    /// entities and events cleanup, and `OnExit`) occur within schedule [`StateTransition`](crate::prelude::StateTransition).
    fn add_event_cleared_on_exit_state<E: Event>(&mut self, state: impl States) -> &mut Self;
}

impl StateScopedEventsAppExt for App {
    fn add_event_cleared_on_exit_state<E: Event>(&mut self, state: impl States) -> &mut Self {
        add_event_cleared_on_exit_state(self.main_mut(), PhantomData::<E>, state);
        self
    }
}

impl StateScopedEventsAppExt for SubApp {
    fn add_event_cleared_on_exit_state<E: Event>(&mut self, state: impl States) -> &mut Self {
        add_event_cleared_on_exit_state(self, PhantomData::<E>, state);
        self
    }
}
