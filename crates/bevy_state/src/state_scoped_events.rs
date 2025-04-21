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

use crate::state::{FreelyMutableState, OnExit, StateTransitionEvent};

fn clear_event_queue<E: Event>(w: &mut World) {
    if let Some(mut queue) = w.get_resource_mut::<Events<E>>() {
        queue.clear();
    }
}

#[derive(Resource)]
struct StateScopedEvents<S: FreelyMutableState> {
    cleanup_fns: HashMap<S, Vec<fn(&mut World)>>,
}

impl<S: FreelyMutableState> StateScopedEvents<S> {
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

impl<S: FreelyMutableState> Default for StateScopedEvents<S> {
    fn default() -> Self {
        Self {
            cleanup_fns: HashMap::default(),
        }
    }
}

fn cleanup_state_scoped_event<S: FreelyMutableState>(
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
        w.resource_scope::<StateScopedEvents<S>, ()>(|w, events| {
            events.cleanup(w, exited);
        });
    });
}

fn add_state_scoped_event_impl<E: Event, S: FreelyMutableState>(
    app: &mut SubApp,
    _p: PhantomData<E>,
    state: S,
) {
    if !app.world().contains_resource::<StateScopedEvents<S>>() {
        app.init_resource::<StateScopedEvents<S>>();
    }
    app.add_event::<E>();
    app.world_mut()
        .resource_mut::<StateScopedEvents<S>>()
        .add_event::<E>(state.clone());
    app.add_systems(OnExit(state), cleanup_state_scoped_event::<S>);
}

/// Extension trait for [`App`] adding methods for registering state scoped events.
pub trait StateScopedEventsAppExt {
    /// Adds an [`Event`] that is automatically cleaned up when leaving the specified `state`.
    ///
    /// Note that event cleanup is ordered ambiguously relative to [`StateScoped`](crate::prelude::StateScoped) entity
    /// cleanup and the [`OnExit`] schedule for the target state. All of these (state scoped
    /// entities and events cleanup, and `OnExit`) occur within schedule [`StateTransition`](crate::prelude::StateTransition)
    /// and system set `StateTransitionSteps::ExitSchedules`.
    fn add_state_scoped_event<E: Event>(&mut self, state: impl FreelyMutableState) -> &mut Self;
}

impl StateScopedEventsAppExt for App {
    fn add_state_scoped_event<E: Event>(&mut self, state: impl FreelyMutableState) -> &mut Self {
        add_state_scoped_event_impl(self.main_mut(), PhantomData::<E>, state);
        self
    }
}

impl StateScopedEventsAppExt for SubApp {
    fn add_state_scoped_event<E: Event>(&mut self, state: impl FreelyMutableState) -> &mut Self {
        add_state_scoped_event_impl(self, PhantomData::<E>, state);
        self
    }
}
