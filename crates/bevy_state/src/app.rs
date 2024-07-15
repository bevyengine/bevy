use bevy_app::{App, MainScheduleOrder, Plugin, PreStartup, PreUpdate, SubApp};
use bevy_ecs::{event::Events, schedule::IntoSystemConfigs, world::FromWorld};
use bevy_utils::{tracing::warn, warn_once};

use crate::state::{
    setup_state_transitions_in_world, ComputedStates, FreelyMutableState, NextState, State,
    StateTransition, StateTransitionEvent, StateTransitionSteps, States, SubStates,
};
use crate::state_scoped::clear_state_scoped_entities;

/// State installation methods for [`App`](bevy_app::App) and [`SubApp`](bevy_app::SubApp).
pub trait AppExtStates {
    /// Initializes a [`State`] with standard starting values.
    ///
    /// This method is idempotent: it has no effect when called again using the same generic type.
    ///
    /// Adds [`State<S>`] and [`NextState<S>`] resources, and enables use of the [`OnEnter`], [`OnTransition`] and [`OnExit`] schedules.
    /// These schedules are triggered before [`Update`](crate::Update) and at startup.
    ///
    /// If you would like to control how other systems run based on the current state, you can
    /// emulate this behavior using the [`in_state`] [`Condition`].
    ///
    /// Note that you can also apply state transitions at other points in the schedule
    /// by triggering the [`StateTransition`](`bevy_ecs::schedule::StateTransition`) schedule manually.
    fn init_state<S: FreelyMutableState + FromWorld>(&mut self) -> &mut Self;

    /// Inserts a specific [`State`] to the current [`App`] and overrides any [`State`] previously
    /// added of the same type.
    ///
    /// Adds [`State<S>`] and [`NextState<S>`] resources, and enables use of the [`OnEnter`], [`OnTransition`] and [`OnExit`] schedules.
    /// These schedules are triggered before [`Update`](crate::Update) and at startup.
    ///
    /// If you would like to control how other systems run based on the current state, you can
    /// emulate this behavior using the [`in_state`] [`Condition`].
    ///
    /// Note that you can also apply state transitions at other points in the schedule
    /// by triggering the [`StateTransition`](`bevy_ecs::schedule::StateTransition`) schedule manually.
    fn insert_state<S: FreelyMutableState>(&mut self, state: S) -> &mut Self;

    /// Sets up a type implementing [`ComputedStates`].
    ///
    /// This method is idempotent: it has no effect when called again using the same generic type.
    fn add_computed_state<S: ComputedStates>(&mut self) -> &mut Self;

    /// Sets up a type implementing [`SubStates`].
    ///
    /// This method is idempotent: it has no effect when called again using the same generic type.
    fn add_sub_state<S: SubStates>(&mut self) -> &mut Self;

    /// Enable state-scoped entity clearing for state `S`.
    ///
    /// For more information refer to [`StateScoped`](crate::state_scoped::StateScoped).
    fn enable_state_scoped_entities<S: States>(&mut self) -> &mut Self;
}

/// Separate function to only warn once for all state installation methods.
fn warn_if_no_states_plugin_installed(app: &SubApp) {
    if !app.is_plugin_added::<StatesPlugin>() {
        warn_once!("States were added to the app, but `StatesPlugin` is not installed.");
    }
}

impl AppExtStates for SubApp {
    fn init_state<S: FreelyMutableState + FromWorld>(&mut self) -> &mut Self {
        warn_if_no_states_plugin_installed(self);
        if !self.world().contains_resource::<State<S>>() {
            self.init_resource::<State<S>>()
                .init_resource::<NextState<S>>()
                .add_event::<StateTransitionEvent<S>>();
            let schedule = self.get_schedule_mut(StateTransition).unwrap();
            S::register_state(schedule);
            let state = self.world().resource::<State<S>>().get().clone();
            self.world_mut().send_event(StateTransitionEvent {
                exited: None,
                entered: Some(state),
            });
        } else {
            let name = std::any::type_name::<S>();
            warn!("State {} is already initialized.", name);
        }

        self
    }

    fn insert_state<S: FreelyMutableState>(&mut self, state: S) -> &mut Self {
        warn_if_no_states_plugin_installed(self);
        if !self.world().contains_resource::<State<S>>() {
            self.insert_resource::<State<S>>(State::new(state.clone()))
                .init_resource::<NextState<S>>()
                .add_event::<StateTransitionEvent<S>>();
            let schedule = self.get_schedule_mut(StateTransition).unwrap();
            S::register_state(schedule);
            self.world_mut().send_event(StateTransitionEvent {
                exited: None,
                entered: Some(state),
            });
        } else {
            // Overwrite previous state and initial event
            self.insert_resource::<State<S>>(State::new(state.clone()));
            self.world_mut()
                .resource_mut::<Events<StateTransitionEvent<S>>>()
                .clear();
            self.world_mut().send_event(StateTransitionEvent {
                exited: None,
                entered: Some(state),
            });
        }

        self
    }

    fn add_computed_state<S: ComputedStates>(&mut self) -> &mut Self {
        warn_if_no_states_plugin_installed(self);
        if !self
            .world()
            .contains_resource::<Events<StateTransitionEvent<S>>>()
        {
            self.add_event::<StateTransitionEvent<S>>();
            let schedule = self.get_schedule_mut(StateTransition).unwrap();
            S::register_computed_state_systems(schedule);
            let state = self
                .world()
                .get_resource::<State<S>>()
                .map(|s| s.get().clone());
            self.world_mut().send_event(StateTransitionEvent {
                exited: None,
                entered: state,
            });
        } else {
            let name = std::any::type_name::<S>();
            warn!("Computed state {} is already initialized.", name);
        }

        self
    }

    fn add_sub_state<S: SubStates>(&mut self) -> &mut Self {
        warn_if_no_states_plugin_installed(self);
        if !self
            .world()
            .contains_resource::<Events<StateTransitionEvent<S>>>()
        {
            self.init_resource::<NextState<S>>();
            self.add_event::<StateTransitionEvent<S>>();
            let schedule = self.get_schedule_mut(StateTransition).unwrap();
            S::register_sub_state_systems(schedule);
            let state = self
                .world()
                .get_resource::<State<S>>()
                .map(|s| s.get().clone());
            self.world_mut().send_event(StateTransitionEvent {
                exited: None,
                entered: state,
            });
        } else {
            let name = std::any::type_name::<S>();
            warn!("Sub state {} is already initialized.", name);
        }

        self
    }

    fn enable_state_scoped_entities<S: States>(&mut self) -> &mut Self {
        if !self
            .world()
            .contains_resource::<Events<StateTransitionEvent<S>>>()
        {
            let name = std::any::type_name::<S>();
            warn!("State scoped entities are enabled for state `{}`, but the state isn't installed in the app!", name);
        }
        // We work with [`StateTransition`] in set [`StateTransitionSteps::ExitSchedules`] as opposed to [`OnExit`],
        // because [`OnExit`] only runs for one specific variant of the state.
        self.add_systems(
            StateTransition,
            clear_state_scoped_entities::<S>.in_set(StateTransitionSteps::ExitSchedules),
        )
    }
}

impl AppExtStates for App {
    fn init_state<S: FreelyMutableState + FromWorld>(&mut self) -> &mut Self {
        self.main_mut().init_state::<S>();
        self
    }

    fn insert_state<S: FreelyMutableState>(&mut self, state: S) -> &mut Self {
        self.main_mut().insert_state::<S>(state);
        self
    }

    fn add_computed_state<S: ComputedStates>(&mut self) -> &mut Self {
        self.main_mut().add_computed_state::<S>();
        self
    }

    fn add_sub_state<S: SubStates>(&mut self) -> &mut Self {
        self.main_mut().add_sub_state::<S>();
        self
    }

    fn enable_state_scoped_entities<S: States>(&mut self) -> &mut Self {
        self.main_mut().enable_state_scoped_entities::<S>();
        self
    }
}

/// Registers the [`StateTransition`] schedule in the [`MainScheduleOrder`] to enable state processing.
pub struct StatesPlugin;

impl Plugin for StatesPlugin {
    fn build(&self, app: &mut App) {
        let mut schedule = app.world_mut().resource_mut::<MainScheduleOrder>();
        schedule.insert_after(PreUpdate, StateTransition);
        schedule.insert_startup_before(PreStartup, StateTransition);
        setup_state_transitions_in_world(app.world_mut());
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        self as bevy_state,
        app::StatesPlugin,
        state::{State, StateTransition, StateTransitionEvent},
    };
    use bevy_app::App;
    use bevy_ecs::event::Events;
    use bevy_state_macros::States;

    use super::AppExtStates;

    #[derive(States, Default, PartialEq, Eq, Hash, Debug, Clone)]
    enum TestState {
        #[default]
        A,
        B,
        C,
    }

    #[test]
    fn insert_state_can_overwrite_init_state() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.init_state::<TestState>();
        app.insert_state(TestState::B);

        let world = app.world_mut();
        world.run_schedule(StateTransition);

        assert_eq!(world.resource::<State<TestState>>().0, TestState::B);
        let events = world.resource::<Events<StateTransitionEvent<TestState>>>();
        assert_eq!(events.len(), 1);
        let mut reader = events.get_reader();
        let last = reader.read(events).last().unwrap();
        assert_eq!(last.exited, None);
        assert_eq!(last.entered, Some(TestState::B));
    }

    #[test]
    fn insert_state_can_overwrite_insert_state() {
        let mut app = App::new();
        app.add_plugins(StatesPlugin);

        app.insert_state(TestState::B);
        app.insert_state(TestState::C);

        let world = app.world_mut();
        world.run_schedule(StateTransition);

        assert_eq!(world.resource::<State<TestState>>().0, TestState::C);
        let events = world.resource::<Events<StateTransitionEvent<TestState>>>();
        assert_eq!(events.len(), 1);
        let mut reader = events.get_reader();
        let last = reader.read(events).last().unwrap();
        assert_eq!(last.exited, None);
        assert_eq!(last.entered, Some(TestState::C));
    }
}
