use bevy_app::{App, MainScheduleOrder, Plugin, PreStartup, PreUpdate, SubApp};
use bevy_ecs::{event::Events, schedule::IntoScheduleConfigs, world::FromWorld};
use bevy_utils::once;
use log::warn;

use crate::{
    state::{
        setup_state_transitions_in_world, ComputedStates, FreelyMutableState, NextState, State,
        StateTransition, StateTransitionEvent, StateTransitionSystems, States, SubStates,
    },
    state_scoped::{despawn_entities_on_enter_state, despawn_entities_on_exit_state},
};

#[cfg(feature = "bevy_reflect")]
use bevy_reflect::{FromReflect, GetTypeRegistration, Typed};

/// State installation methods for [`App`] and [`SubApp`].
pub trait AppExtStates {
    /// Initializes a [`State`] with standard starting values.
    ///
    /// This method is idempotent: it has no effect when called again using the same generic type.
    ///
    /// Adds [`State<S>`] and [`NextState<S>`] resources, and enables use of the [`OnEnter`](crate::state::OnEnter),
    /// [`OnTransition`](crate::state::OnTransition) and [`OnExit`](crate::state::OnExit) schedules.
    /// These schedules are triggered before [`Update`](bevy_app::Update) and at startup.
    ///
    /// If you would like to control how other systems run based on the current state, you can
    /// emulate this behavior using the [`in_state`](crate::condition::in_state) [`SystemCondition`](bevy_ecs::prelude::SystemCondition).
    ///
    /// Note that you can also apply state transitions at other points in the schedule
    /// by triggering the [`StateTransition`](struct@StateTransition) schedule manually.
    ///
    /// The use of any states requires the presence of [`StatesPlugin`] (which is included in `DefaultPlugins`).
    fn init_state<S: FreelyMutableState + FromWorld>(&mut self) -> &mut Self;

    /// Inserts a specific [`State`] to the current [`App`] and overrides any [`State`] previously
    /// added of the same type.
    ///
    /// Adds [`State<S>`] and [`NextState<S>`] resources, and enables use of the [`OnEnter`](crate::state::OnEnter),
    /// [`OnTransition`](crate::state::OnTransition) and [`OnExit`](crate::state::OnExit) schedules.
    /// These schedules are triggered before [`Update`](bevy_app::Update) and at startup.
    ///
    /// If you would like to control how other systems run based on the current state, you can
    /// emulate this behavior using the [`in_state`](crate::condition::in_state) [`SystemCondition`](bevy_ecs::prelude::SystemCondition).
    ///
    /// Note that you can also apply state transitions at other points in the schedule
    /// by triggering the [`StateTransition`](struct@StateTransition) schedule manually.
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
    /// This is enabled by default. If you don't want this behavior, add the `#[states(scoped_entities = false)]`
    /// attribute when deriving the [`States`] trait.
    ///
    /// For more information refer to [`crate::state_scoped`].
    #[doc(hidden)]
    fn enable_state_scoped_entities<S: States>(&mut self) -> &mut Self;

    #[cfg(feature = "bevy_reflect")]
    /// Registers the state type `T` using [`App::register_type`],
    /// and adds [`ReflectState`](crate::reflect::ReflectState) type data to `T` in the type registry.
    ///
    /// This enables reflection code to access the state. For detailed information, see the docs on [`crate::reflect::ReflectState`] .
    fn register_type_state<S>(&mut self) -> &mut Self
    where
        S: States + FromReflect + GetTypeRegistration + Typed;

    #[cfg(feature = "bevy_reflect")]
    /// Registers the state type `T` using [`App::register_type`],
    /// and adds [`crate::reflect::ReflectState`] and [`crate::reflect::ReflectFreelyMutableState`] type data to `T` in the type registry.
    ///
    /// This enables reflection code to access and modify the state.
    /// For detailed information, see the docs on [`crate::reflect::ReflectState`] and [`crate::reflect::ReflectFreelyMutableState`].
    fn register_type_mutable_state<S>(&mut self) -> &mut Self
    where
        S: FreelyMutableState + FromReflect + GetTypeRegistration + Typed;
}

/// Separate function to only warn once for all state installation methods.
fn warn_if_no_states_plugin_installed(app: &SubApp) {
    if !app.is_plugin_added::<StatesPlugin>() {
        once!(warn!(
            "States were added to the app, but `StatesPlugin` is not installed."
        ));
    }
}

impl AppExtStates for SubApp {
    fn init_state<S: FreelyMutableState + FromWorld>(&mut self) -> &mut Self {
        warn_if_no_states_plugin_installed(self);
        if !self.world().contains_resource::<State<S>>() {
            self.init_resource::<State<S>>()
                .init_resource::<NextState<S>>()
                .add_event::<StateTransitionEvent<S>>();
            let schedule = self.get_schedule_mut(StateTransition).expect(
                "The `StateTransition` schedule is missing. Did you forget to add StatesPlugin or DefaultPlugins before calling init_state?"
            );
            S::register_state(schedule);
            let state = self.world().resource::<State<S>>().get().clone();
            self.world_mut().write_event(StateTransitionEvent {
                exited: None,
                entered: Some(state),
            });
            if S::SCOPED_ENTITIES_ENABLED {
                self.enable_state_scoped_entities::<S>();
            }
        } else {
            let name = core::any::type_name::<S>();
            warn!("State {name} is already initialized.");
        }

        self
    }

    fn insert_state<S: FreelyMutableState>(&mut self, state: S) -> &mut Self {
        warn_if_no_states_plugin_installed(self);
        if !self.world().contains_resource::<State<S>>() {
            self.insert_resource::<State<S>>(State::new(state.clone()))
                .init_resource::<NextState<S>>()
                .add_event::<StateTransitionEvent<S>>();
            let schedule = self.get_schedule_mut(StateTransition).expect(
                "The `StateTransition` schedule is missing. Did you forget to add StatesPlugin or DefaultPlugins before calling insert_state?"
            );
            S::register_state(schedule);
            self.world_mut().write_event(StateTransitionEvent {
                exited: None,
                entered: Some(state),
            });
            if S::SCOPED_ENTITIES_ENABLED {
                self.enable_state_scoped_entities::<S>();
            }
        } else {
            // Overwrite previous state and initial event
            self.insert_resource::<State<S>>(State::new(state.clone()));
            self.world_mut()
                .resource_mut::<Events<StateTransitionEvent<S>>>()
                .clear();
            self.world_mut().write_event(StateTransitionEvent {
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
            let schedule = self.get_schedule_mut(StateTransition).expect(
                "The `StateTransition` schedule is missing. Did you forget to add StatesPlugin or DefaultPlugins before calling add_computed_state?"
            );
            S::register_computed_state_systems(schedule);
            let state = self
                .world()
                .get_resource::<State<S>>()
                .map(|s| s.get().clone());
            self.world_mut().write_event(StateTransitionEvent {
                exited: None,
                entered: state,
            });
            if S::SCOPED_ENTITIES_ENABLED {
                self.enable_state_scoped_entities::<S>();
            }
        } else {
            let name = core::any::type_name::<S>();
            warn!("Computed state {name} is already initialized.");
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
            let schedule = self.get_schedule_mut(StateTransition).expect(
                "The `StateTransition` schedule is missing. Did you forget to add StatesPlugin or DefaultPlugins before calling add_sub_state?"
            );
            S::register_sub_state_systems(schedule);
            let state = self
                .world()
                .get_resource::<State<S>>()
                .map(|s| s.get().clone());
            self.world_mut().write_event(StateTransitionEvent {
                exited: None,
                entered: state,
            });
            if S::SCOPED_ENTITIES_ENABLED {
                self.enable_state_scoped_entities::<S>();
            }
        } else {
            let name = core::any::type_name::<S>();
            warn!("Sub state {name} is already initialized.");
        }

        self
    }

    #[doc(hidden)]
    fn enable_state_scoped_entities<S: States>(&mut self) -> &mut Self {
        if !self
            .world()
            .contains_resource::<Events<StateTransitionEvent<S>>>()
        {
            let name = core::any::type_name::<S>();
            warn!("State scoped entities are enabled for state `{name}`, but the state isn't installed in the app!");
        }

        // Note: We work with `StateTransition` in set
        // `StateTransitionSystems::ExitSchedules` rather than `OnExit`, because
        // `OnExit` only runs for one specific variant of the state.
        self.add_systems(
            StateTransition,
            despawn_entities_on_exit_state::<S>.in_set(StateTransitionSystems::ExitSchedules),
        )
        // Note: We work with `StateTransition` in set
        // `StateTransitionSystems::EnterSchedules` rather than `OnEnter`, because
        // `OnEnter` only runs for one specific variant of the state.
        .add_systems(
            StateTransition,
            despawn_entities_on_enter_state::<S>.in_set(StateTransitionSystems::EnterSchedules),
        )
    }

    #[cfg(feature = "bevy_reflect")]
    fn register_type_state<S>(&mut self) -> &mut Self
    where
        S: States + FromReflect + GetTypeRegistration + Typed,
    {
        self.register_type::<S>();
        self.register_type::<State<S>>();
        self.register_type_data::<S, crate::reflect::ReflectState>();
        self
    }

    #[cfg(feature = "bevy_reflect")]
    fn register_type_mutable_state<S>(&mut self) -> &mut Self
    where
        S: FreelyMutableState + FromReflect + GetTypeRegistration + Typed,
    {
        self.register_type::<S>();
        self.register_type::<State<S>>();
        self.register_type::<NextState<S>>();
        self.register_type_data::<S, crate::reflect::ReflectState>();
        self.register_type_data::<S, crate::reflect::ReflectFreelyMutableState>();
        self
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

    #[doc(hidden)]
    fn enable_state_scoped_entities<S: States>(&mut self) -> &mut Self {
        self.main_mut().enable_state_scoped_entities::<S>();
        self
    }

    #[cfg(feature = "bevy_reflect")]
    fn register_type_state<S>(&mut self) -> &mut Self
    where
        S: States + FromReflect + GetTypeRegistration + Typed,
    {
        self.main_mut().register_type_state::<S>();
        self
    }

    #[cfg(feature = "bevy_reflect")]
    fn register_type_mutable_state<S>(&mut self) -> &mut Self
    where
        S: FreelyMutableState + FromReflect + GetTypeRegistration + Typed,
    {
        self.main_mut().register_type_mutable_state::<S>();
        self
    }
}

/// Registers the [`StateTransition`] schedule in the [`MainScheduleOrder`] to enable state processing.
#[derive(Default)]
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
        let mut reader = events.get_cursor();
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
        let mut reader = events.get_cursor();
        let last = reader.read(events).last().unwrap();
        assert_eq!(last.exited, None);
        assert_eq!(last.entered, Some(TestState::C));
    }
}
