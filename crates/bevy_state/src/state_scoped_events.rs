use alloc::vec::Vec;
use core::marker::PhantomData;

use bevy_app::{App, SubApp};
use bevy_ecs::{
    message::{Message, MessageReader, Messages},
    resource::Resource,
    system::Commands,
    world::World,
};
use bevy_platform::collections::HashMap;

use crate::state::{OnEnter, OnExit, StateTransitionEvent, States};

fn clear_message_queue<M: Message>(w: &mut World) {
    if let Some(mut queue) = w.get_resource_mut::<Messages<M>>() {
        queue.clear();
    }
}

#[derive(Copy, Clone)]
enum TransitionType {
    OnExit,
    OnEnter,
}

#[derive(Resource)]
struct StateScopedMessages<S: States> {
    /// Keeps track of which messages need to be reset when the state is exited.
    on_exit: HashMap<S, Vec<fn(&mut World)>>,
    /// Keeps track of which messages need to be reset when the state is entered.
    on_enter: HashMap<S, Vec<fn(&mut World)>>,
}

impl<S: States> StateScopedMessages<S> {
    fn add_message<M: Message>(&mut self, state: S, transition_type: TransitionType) {
        let map = match transition_type {
            TransitionType::OnExit => &mut self.on_exit,
            TransitionType::OnEnter => &mut self.on_enter,
        };
        map.entry(state).or_default().push(clear_message_queue::<M>);
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

impl<S: States> Default for StateScopedMessages<S> {
    fn default() -> Self {
        Self {
            on_exit: HashMap::default(),
            on_enter: HashMap::default(),
        }
    }
}

fn clear_messages_on_exit<S: States>(
    mut c: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
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
        w.resource_scope::<StateScopedMessages<S>, ()>(|w, messages| {
            messages.cleanup(w, exited, TransitionType::OnExit);
        });
    });
}

fn clear_messages_on_enter<S: States>(
    mut c: Commands,
    mut transitions: MessageReader<StateTransitionEvent<S>>,
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
        w.resource_scope::<StateScopedMessages<S>, ()>(|w, messages| {
            messages.cleanup(w, entered, TransitionType::OnEnter);
        });
    });
}

fn clear_messages_on_state_transition<M: Message, S: States>(
    app: &mut SubApp,
    _p: PhantomData<M>,
    state: S,
    transition_type: TransitionType,
) {
    if !app.world().contains_resource::<StateScopedMessages<S>>() {
        app.init_resource::<StateScopedMessages<S>>();
    }
    app.world_mut()
        .resource_mut::<StateScopedMessages<S>>()
        .add_message::<M>(state.clone(), transition_type);
    match transition_type {
        TransitionType::OnExit => app.add_systems(OnExit(state), clear_messages_on_exit::<S>),
        TransitionType::OnEnter => app.add_systems(OnEnter(state), clear_messages_on_enter::<S>),
    };
}

/// Extension trait for [`App`] adding methods for registering state scoped messages.
pub trait StateScopedMessagesAppExt {
    /// Clears a [`Message`] when exiting the specified `state`.
    ///
    /// Note that message cleanup is ambiguously ordered relative to
    /// [`DespawnOnExit`](crate::prelude::DespawnOnExit) entity cleanup,
    /// and the [`OnExit`] schedule for the target state.
    /// All of these (state scoped entities and messages cleanup, and `OnExit`)
    /// occur within schedule [`StateTransition`](crate::prelude::StateTransition)
    /// and system set `StateTransitionSystems::ExitSchedules`.
    fn clear_messages_on_exit<M: Message>(&mut self, state: impl States) -> &mut Self;

    /// Clears a [`Message`] when entering the specified `state`.
    ///
    /// Note that message cleanup is ambiguously ordered relative to
    /// [`DespawnOnEnter`](crate::prelude::DespawnOnEnter) entity cleanup,
    /// and the [`OnEnter`] schedule for the target state.
    /// All of these (state scoped entities and messages cleanup, and `OnEnter`)
    /// occur within schedule [`StateTransition`](crate::prelude::StateTransition)
    /// and system set `StateTransitionSystems::EnterSchedules`.
    fn clear_messages_on_enter<M: Message>(&mut self, state: impl States) -> &mut Self;
}

impl StateScopedMessagesAppExt for App {
    fn clear_messages_on_exit<M: Message>(&mut self, state: impl States) -> &mut Self {
        clear_messages_on_state_transition(
            self.main_mut(),
            PhantomData::<M>,
            state,
            TransitionType::OnExit,
        );
        self
    }

    fn clear_messages_on_enter<M: Message>(&mut self, state: impl States) -> &mut Self {
        clear_messages_on_state_transition(
            self.main_mut(),
            PhantomData::<M>,
            state,
            TransitionType::OnEnter,
        );
        self
    }
}

impl StateScopedMessagesAppExt for SubApp {
    fn clear_messages_on_exit<M: Message>(&mut self, state: impl States) -> &mut Self {
        clear_messages_on_state_transition(self, PhantomData::<M>, state, TransitionType::OnExit);
        self
    }

    fn clear_messages_on_enter<M: Message>(&mut self, state: impl States) -> &mut Self {
        clear_messages_on_state_transition(self, PhantomData::<M>, state, TransitionType::OnEnter);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::StatesPlugin;
    use bevy_ecs::message::Message;
    use bevy_state::prelude::*;

    #[derive(States, Default, Clone, Hash, Eq, PartialEq, Debug)]
    enum TestState {
        #[default]
        A,
        B,
    }

    #[derive(Message, Debug)]
    struct StandardMessage;

    #[derive(Message, Debug)]
    struct StateScopedMessage;

    #[test]
    fn clear_message_on_exit_state() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<TestState>();

        app.add_message::<StandardMessage>();
        app.add_message::<StateScopedMessage>()
            .clear_messages_on_exit::<StateScopedMessage>(TestState::A);

        app.world_mut().write_message(StandardMessage).unwrap();
        app.world_mut().write_message(StateScopedMessage).unwrap();
        assert!(!app
            .world()
            .resource::<Messages<StandardMessage>>()
            .is_empty());
        assert!(!app
            .world()
            .resource::<Messages<StateScopedMessage>>()
            .is_empty());

        app.world_mut()
            .resource_mut::<NextState<TestState>>()
            .set(TestState::B);
        app.update();

        assert!(!app
            .world()
            .resource::<Messages<StandardMessage>>()
            .is_empty());
        assert!(app
            .world()
            .resource::<Messages<StateScopedMessage>>()
            .is_empty());
    }

    #[test]
    fn clear_message_on_enter_state() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);
        app.init_state::<TestState>();

        app.add_message::<StandardMessage>();
        app.add_message::<StateScopedMessage>()
            .clear_messages_on_enter::<StateScopedMessage>(TestState::B);

        app.world_mut().write_message(StandardMessage).unwrap();
        app.world_mut().write_message(StateScopedMessage).unwrap();
        assert!(!app
            .world()
            .resource::<Messages<StandardMessage>>()
            .is_empty());
        assert!(!app
            .world()
            .resource::<Messages<StateScopedMessage>>()
            .is_empty());

        app.world_mut()
            .resource_mut::<NextState<TestState>>()
            .set(TestState::B);
        app.update();

        assert!(!app
            .world()
            .resource::<Messages<StandardMessage>>()
            .is_empty());
        assert!(app
            .world()
            .resource::<Messages<StateScopedMessage>>()
            .is_empty());
    }
}
