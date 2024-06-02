use bevy_app::{App, Startup, SubApp};
use bevy_ecs::{event::Events, schedule::ScheduleLabel, world::FromWorld};

use crate::state::{
    setup_state_transitions_in_world, ComputedStates, FreelyMutableState, NextState, State,
    StateTransition, StateTransitionEvent, SubStates,
};

/// State installation methods for [`App`](bevy_app::App) and [`SubApp`](bevy_app::SubApp).
pub trait AppStateExt {
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
}

impl AppStateExt for SubApp {
    /// See [`App::init_state`].
    fn init_state<S: FreelyMutableState + FromWorld>(&mut self) -> &mut Self {
        if !self.world().contains_resource::<State<S>>() {
            setup_state_transitions_in_world(self.world_mut(), Some(Startup.intern()));
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
        }

        self
    }

    /// See [`App::insert_state`].
    fn insert_state<S: FreelyMutableState>(&mut self, state: S) -> &mut Self {
        if !self.world().contains_resource::<State<S>>() {
            setup_state_transitions_in_world(self.world_mut(), Some(Startup.intern()));
            self.insert_resource::<State<S>>(State::new(state.clone()))
                .init_resource::<NextState<S>>()
                .add_event::<StateTransitionEvent<S>>();
            let schedule = self.get_schedule_mut(StateTransition).unwrap();
            S::register_state(schedule);
            self.world_mut().send_event(StateTransitionEvent {
                exited: None,
                entered: Some(state),
            });
        }

        self
    }

    /// See [`App::add_computed_state`].
    fn add_computed_state<S: ComputedStates>(&mut self) -> &mut Self {
        if !self
            .world()
            .contains_resource::<Events<StateTransitionEvent<S>>>()
        {
            setup_state_transitions_in_world(self.world_mut(), Some(Startup.intern()));
            self.add_event::<StateTransitionEvent<S>>();
            let schedule = self.get_schedule_mut(StateTransition).unwrap();
            S::register_computed_state_systems(schedule);
            let state = self.world().resource::<State<S>>().get().clone();
            self.world_mut().send_event(StateTransitionEvent {
                exited: None,
                entered: Some(state),
            });
        }

        self
    }

    /// See [`App::add_sub_state`].
    fn add_sub_state<S: SubStates>(&mut self) -> &mut Self {
        if !self
            .world()
            .contains_resource::<Events<StateTransitionEvent<S>>>()
        {
            setup_state_transitions_in_world(self.world_mut(), Some(Startup.intern()));
            self.init_resource::<NextState<S>>();
            self.add_event::<StateTransitionEvent<S>>();
            let schedule = self.get_schedule_mut(StateTransition).unwrap();
            S::register_sub_state_systems(schedule);
            let state = self.world().resource::<State<S>>().get().clone();
            self.world_mut().send_event(StateTransitionEvent {
                exited: None,
                entered: Some(state),
            });
        }

        self
    }
}

impl AppStateExt for App {
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
}
