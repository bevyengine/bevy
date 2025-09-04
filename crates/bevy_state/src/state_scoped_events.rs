use alloc::vec::Vec;
use core::marker::PhantomData;

use bevy_app::{App, SubApp};
use bevy_ecs::{
    event::{BufferedEvent, EventReader, Events},
    resource::Resource,
    system::Commands,
    world::World,
};
use bevy_platform::collections::HashMap;

use crate::state::{OnEnter, OnExit, StateTransitionEvent, States};

fn clear_event_queue<E: BufferedEvent>(w: &mut World) {
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
struct StateScopedEvents<S: States> {
    /// Keeps track of which events need to be reset when the state is exited.
    on_exit: HashMap<S, Vec<fn(&mut World)>>,
    /// Keeps track of which events need to be reset when the state is entered.
    on_enter: HashMap<S, Vec<fn(&mut World)>>,
}

impl<S: States> StateScopedEvents<S> {
    fn add_event<E: BufferedEvent>(&mut self, state: S, transition_type: TransitionType) {
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

impl<S: States> Default for StateScopedEvents<S> {
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
        w.resource_scope::<StateScopedEvents<S>, ()>(|w, events| {
            events.cleanup(w, exited, TransitionType::OnExit);
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
        w.resource_scope::<StateScopedEvents<S>, ()>(|w, events| {
            events.cleanup(w, entered, TransitionType::OnEnter);
        });
    });
}

fn clear_events_on_state_transition<E: BufferedEvent, S: States>(
    app: &mut SubApp,
    _p: PhantomData<E>,
    state: S,
    transition_type: TransitionType,
) {
    if !app.world().contains_resource::<StateScopedEvents<S>>() {
        app.init_resource::<StateScopedEvents<S>>();
    }
    app.world_mut()
        .resource_mut::<StateScopedEvents<S>>()
        .add_event::<E>(state.clone(), transition_type);
    match transition_type {
        TransitionType::OnExit => app.add_systems(OnExit(state), clear_events_on_exit_state::<S>),
        TransitionType::OnEnter => {
            app.add_systems(OnEnter(state), clear_events_on_enter_state::<S>)
        }
    };
}

/// Extension trait for [`App`] adding methods for registering state scoped events.
pub trait StateScopedEventsAppExt {
    /// Clears an [`BufferedEvent`] when exiting the specified `state`.
    ///
    /// Note that event cleanup is ambiguously ordered relative to
    /// [`DespawnOnExit`](crate::prelude::DespawnOnExit) entity cleanup,
    /// and the [`OnExit`] schedule for the target state.
    /// All of these (state scoped entities and events cleanup, and `OnExit`)
    /// occur within schedule [`StateTransition`](crate::prelude::StateTransition)
    /// and system set `StateTransitionSystems::ExitSchedules`.
    fn clear_events_on_exit_state<E: BufferedEvent>(&mut self, state: impl States) -> &mut Self;

    /// Clears an [`BufferedEvent`] when entering the specified `state`.
    ///
    /// Note that event cleanup is ambiguously ordered relative to
    /// [`DespawnOnEnter`](crate::prelude::DespawnOnEnter) entity cleanup,
    /// and the [`OnEnter`] schedule for the target state.
    /// All of these (state scoped entities and events cleanup, and `OnEnter`)
    /// occur within schedule [`StateTransition`](crate::prelude::StateTransition)
    /// and system set `StateTransitionSystems::EnterSchedules`.
    fn clear_events_on_enter_state<E: BufferedEvent>(&mut self, state: impl States) -> &mut Self;
}

impl StateScopedEventsAppExt for App {
    fn clear_events_on_exit_state<E: BufferedEvent>(&mut self, state: impl States) -> &mut Self {
        clear_events_on_state_transition(
            self.main_mut(),
            PhantomData::<E>,
            state,
            TransitionType::OnExit,
        );
        self
    }

    fn clear_events_on_enter_state<E: BufferedEvent>(&mut self, state: impl States) -> &mut Self {
        clear_events_on_state_transition(
            self.main_mut(),
            PhantomData::<E>,
            state,
            TransitionType::OnEnter,
        );
        self
    }
}

impl StateScopedEventsAppExt for SubApp {
    fn clear_events_on_exit_state<E: BufferedEvent>(&mut self, state: impl States) -> &mut Self {
        clear_events_on_state_transition(self, PhantomData::<E>, state, TransitionType::OnExit);
        self
    }

    fn clear_events_on_enter_state<E: BufferedEvent>(&mut self, state: impl States) -> &mut Self {
        clear_events_on_state_transition(self, PhantomData::<E>, state, TransitionType::OnEnter);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::StatesPlugin;
    use bevy_ecs::event::BufferedEvent;
    use bevy_state::prelude::*;

    #[derive(States, Default, Clone, Hash, Eq, PartialEq, Debug)]
    enum TestState {
        #[default]
        A,
        B,
    }

    #[derive(BufferedEvent, Debug)]
    struct StandardEvent;

    #[derive(BufferedEvent, Debug)]
    struct StateScopedEvent;

    #[test]
    fn clear_event_on_exit_state() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<TestState>();

        app.add_event::<StandardEvent>();
        app.add_event::<StateScopedEvent>()
            .clear_events_on_exit_state::<StateScopedEvent>(TestState::A);

        app.world_mut().write_event(StandardEvent).unwrap();
        app.world_mut().write_event(StateScopedEvent).unwrap();
        assert!(!app.world().resource::<Events<StandardEvent>>().is_empty());
        assert!(!app
            .world()
            .resource::<Events<StateScopedEvent>>()
            .is_empty());

        app.world_mut()
            .resource_mut::<NextState<TestState>>()
            .set(TestState::B);
        app.update();

        assert!(!app.world().resource::<Events<StandardEvent>>().is_empty());
        assert!(app
            .world()
            .resource::<Events<StateScopedEvent>>()
            .is_empty());
    }

    #[test]
    fn clear_event_on_enter_state() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<TestState>();

        app.add_event::<StandardEvent>();
        app.add_event::<StateScopedEvent>()
            .clear_events_on_enter_state::<StateScopedEvent>(TestState::B);

        app.world_mut().write_event(StandardEvent).unwrap();
        app.world_mut().write_event(StateScopedEvent).unwrap();
        assert!(!app.world().resource::<Events<StandardEvent>>().is_empty());
        assert!(!app
            .world()
            .resource::<Events<StateScopedEvent>>()
            .is_empty());

        app.world_mut()
            .resource_mut::<NextState<TestState>>()
            .set(TestState::B);
        app.update();

        assert!(!app.world().resource::<Events<StandardEvent>>().is_empty());
        assert!(app
            .world()
            .resource::<Events<StateScopedEvent>>()
            .is_empty());
    }
}
