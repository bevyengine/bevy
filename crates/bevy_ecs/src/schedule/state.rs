use crate::{
    schedule::{
        RunCriteriaDescriptor, RunCriteriaDescriptorCoercion, RunCriteriaLabel, ShouldRun,
        SystemSet,
    },
    system::{In, IntoChainSystem, Local, Res, ResMut},
};
use std::{
    any::TypeId,
    fmt::{self, Debug},
    hash::Hash,
};

pub trait StateData: Send + Sync + Clone + Eq + Debug + Hash + 'static {}
impl<T> StateData for T where T: Send + Sync + Clone + Eq + Debug + Hash + 'static {}

/// A simple finite-state machine whose transitions (enter and exit) can have associated run criteria
/// ([`on_enter`](#method.on_enter) and [`on_exit`](#method.on_exit)).
///
/// A state transition can be scheduled with [`State::set`](#method.set).
#[derive(Debug)]
pub struct State<T: StateData> {
    current_state: T,
    next_state: Option<T>,
    transition: Option<StateTransition<T>>,
    end_next_loop: bool,
}

#[derive(Debug)]
enum StateTransition<T: StateData> {
    PreStartup,
    Startup,
    // The parameter order is always (leaving, entering)
    Exiting(T, T),
    Entering(T, T),
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
struct DriverLabel(TypeId, &'static str);
impl RunCriteriaLabel for DriverLabel {
    fn type_id(&self) -> core::any::TypeId {
        self.0
    }
    fn as_str(&self) -> &'static str {
        self.1
    }
}

impl DriverLabel {
    fn of<T: 'static>() -> Self {
        Self(TypeId::of::<T>(), std::any::type_name::<T>())
    }
}

impl<T> State<T>
where
    T: StateData,
{
    pub fn on_update(pred: T) -> RunCriteriaDescriptor {
        (move |state: Res<State<T>>| state.current_state == pred && state.transition.is_none())
            .chain(should_run_adapter::<T>)
            .after(DriverLabel::of::<T>())
    }

    pub fn on_enter(pred: T) -> RunCriteriaDescriptor {
        (move |state: Res<State<T>>| {
            state
                .transition
                .as_ref()
                .map_or(false, |transition| match transition {
                    StateTransition::Entering(_, entering) => entering == &pred,
                    StateTransition::Startup => state.current_state == pred,
                    _ => false,
                })
        })
        .chain(should_run_adapter::<T>)
        .after(DriverLabel::of::<T>())
    }

    pub fn on_exit(pred: T) -> RunCriteriaDescriptor {
        (move |state: Res<State<T>>| {
            state
                .transition
                .as_ref()
                .map_or(false, |transition| match transition {
                    StateTransition::Exiting(exiting, _) => exiting == &pred,
                    _ => false,
                })
        })
        .chain(should_run_adapter::<T>)
        .after(DriverLabel::of::<T>())
    }

    pub fn on_update_set(s: T) -> SystemSet {
        SystemSet::new().with_run_criteria(Self::on_update(s))
    }

    pub fn on_enter_set(s: T) -> SystemSet {
        SystemSet::new().with_run_criteria(Self::on_enter(s))
    }

    pub fn on_exit_set(s: T) -> SystemSet {
        SystemSet::new().with_run_criteria(Self::on_exit(s))
    }

    /// Creates a driver set for the State.
    ///
    /// Important note: this set must be inserted **before** all other state-dependant sets to work
    /// properly!
    pub fn get_driver() -> SystemSet {
        SystemSet::default().with_run_criteria(state_cleaner::<T>.label(DriverLabel::of::<T>()))
    }

    pub fn new(initial: T) -> Self {
        Self {
            current_state: initial,
            next_state: None,
            transition: Some(StateTransition::PreStartup),
            end_next_loop: false,
        }
    }

    /// Schedule a state change that replaces the active state with the given state.
    /// This will fail if there is a scheduled operation or a pending transition.
    ///
    /// If `state` is the same as the current one, this does nothing.
    /// Use [`restart`](#method.restart) to trigger `on_exit` and `on_enter` for the current state.
    pub fn set(&mut self, state: T) -> Result<(), StateError> {
        if self.next_state.is_some() || self.transition.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        // Only schedule a transition if the passed `state` is distinct from the current one.
        if self.current_state != state {
            self.next_state = Some(state);
        }
        Ok(())
    }

    /// Schedule a state change that replaces the active state with the given state.
    /// Overwrites any previously scheduled transition.
    ///
    /// If `state` is the same as the current one, this does nothing.
    /// Use [`overwrite_restart`](#method.overwrite_restart) to forcibly trigger
    /// [`on_exit`](#method.on_exit) and [`on_enter`](#method.on_enter) for the current state.
    pub fn overwrite_set(&mut self, state: T) {
        // If `state` is distinct from the current one, schedule a transition.
        if self.current_state != state {
            self.next_state = Some(state);
        }
        // We don't need to perform a state transition, but check if we need
        // to cancel any previously scheduled transitions.
        else if self.next_state != Some(state) {
            self.next_state = None;
        }
    }

    /// Schedules a state transition from the current state, back to the current state.
    /// This is useful if you want to forcibly trigger [`on_exit`](#method.on_exit) and
    /// [`on_enter`](#method.on_enter) for the current state.
    ///
    /// This will fail if there is already a scheduled operation or pending transition.
    pub fn restart(&mut self) -> Result<(), StateError> {
        if self.next_state.is_some() || self.transition.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        let state = self.current_state.clone();
        self.next_state = Some(state);
        Ok(())
    }

    /// Schedules a state transition from the current state, back to the current state.
    /// This is useful if you want to forcibly trigger `on_exit` and `on_enter` for the current state.
    pub fn overwrite_restart(&mut self) {
        let state = self.current_state.clone();
        self.next_state = Some(state);
    }

    /// Returns the current state.
    pub fn current(&self) -> &T {
        &self.current_state
    }

    /// Clears any scheduled state operation.
    pub fn clear_schedule(&mut self) {
        self.next_state = None;
    }
}

#[derive(Debug)]
pub enum StateError {
    StateAlreadyQueued,
}

impl std::error::Error for StateError {}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StateError::StateAlreadyQueued => write!(
                f,
                "Attempted to queue a state change, but there was already a state queued."
            ),
        }
    }
}

fn should_run_adapter<T: StateData>(In(cmp_result): In<bool>, state: Res<State<T>>) -> ShouldRun {
    if state.end_next_loop {
        return ShouldRun::No;
    }
    if cmp_result {
        ShouldRun::YesAndCheckAgain
    } else {
        ShouldRun::NoAndCheckAgain
    }
}

fn state_cleaner<T: StateData>(
    mut state: ResMut<State<T>>,
    mut prep_exit: Local<bool>,
) -> ShouldRun {
    if *prep_exit {
        *prep_exit = false;
        if state.next_state.is_none() {
            state.end_next_loop = true;
            return ShouldRun::YesAndCheckAgain;
        }
    } else if state.end_next_loop {
        state.end_next_loop = false;
        return ShouldRun::No;
    }
    match state.next_state.take() {
        Some(next) => {
            state.transition = Some(StateTransition::Exiting(state.current_state.clone(), next));
        }
        None => match state.transition.take() {
            Some(StateTransition::Exiting(p, n)) => {
                state.transition = Some(StateTransition::Entering(p, n.clone()));
                state.current_state = n;
            }
            Some(StateTransition::PreStartup) => {
                state.transition = Some(StateTransition::Startup);
            }
            _ => {}
        },
    };
    if state.transition.is_none() {
        *prep_exit = true;
    }

    ShouldRun::YesAndCheckAgain
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::prelude::*;

    #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
    enum MyState {
        S1,
        S2,
        S3,
        S4,
        S5,
        S6,
        Final,
    }

    #[test]
    fn state_test() {
        let mut world = World::default();

        world.insert_resource(Vec::<&'static str>::new());
        world.insert_resource(State::new(MyState::S1));

        let mut stage = SystemStage::parallel();

        stage.add_system_set(State::<MyState>::get_driver());
        stage
            .add_system_set(
                State::on_enter_set(MyState::S1)
                    .with_system(|mut r: ResMut<Vec<&'static str>>| r.push("startup")),
            )
            .add_system_set(State::on_update_set(MyState::S1).with_system(
                |mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                    r.push("update S1");
                    s.overwrite_set(MyState::S2);
                },
            ))
            .add_system_set(
                State::on_enter_set(MyState::S2)
                    .with_system(|mut r: ResMut<Vec<&'static str>>| r.push("enter S2")),
            )
            .add_system_set(State::on_update_set(MyState::S2).with_system(
                |mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                    r.push("update S2");
                    s.overwrite_set(MyState::S3);
                },
            ))
            .add_system_set(
                State::on_exit_set(MyState::S2)
                    .with_system(|mut r: ResMut<Vec<&'static str>>| r.push("exit S2")),
            )
            .add_system_set(
                State::on_enter_set(MyState::S3)
                    .with_system(|mut r: ResMut<Vec<&'static str>>| r.push("enter S3")),
            )
            .add_system_set(State::on_update_set(MyState::S3).with_system(
                |mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                    r.push("update S3");
                    s.overwrite_set(MyState::S4);
                },
            ))
            .add_system_set(State::on_update_set(MyState::S4).with_system(
                |mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                    r.push("update S4");
                    s.overwrite_set(MyState::S5);
                },
            ))
            .add_system_set(State::on_update_set(MyState::S5).with_system(
                |mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                    r.push("update S5");
                    s.overwrite_set(MyState::S6);
                },
            ))
            .add_system_set(
                State::on_exit_set(MyState::S5)
                    .with_system(|mut r: ResMut<Vec<&'static str>>| r.push("exit S5")),
            )
            .add_system_set(State::on_update_set(MyState::S6).with_system(
                |mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                    r.push("update S6");
                    s.overwrite_set(MyState::Final);
                },
            ));

        const EXPECTED: &[&str] = &[
            //
            "startup",
            "update S1",
            //
            "enter S2",
            "update S2",
            //
            "exit S2",
            "enter S3",
            "update S3",
            //
            "update S4",
            //
            "update S5",
            //
            "exit S5",
            "update S6",
        ];

        stage.run(&mut world);
        let mut collected = world.resource_mut::<Vec<&'static str>>();
        let mut count = 0;
        for (found, expected) in collected.drain(..).zip(EXPECTED) {
            assert_eq!(found, *expected);
            count += 1;
        }
        // If not equal, some elements weren't executed
        assert_eq!(EXPECTED.len(), count);
        assert_eq!(
            world.resource::<State<MyState>>().current(),
            &MyState::Final
        );
    }

    #[test]
    fn issue_1753() {
        #[derive(Clone, PartialEq, Eq, Debug, Hash)]
        enum AppState {
            Main,
        }

        fn should_run_once(mut flag: ResMut<bool>, test_name: Res<&'static str>) {
            assert!(!*flag, "{:?}", *test_name);
            *flag = true;
        }

        let mut world = World::new();
        world.insert_resource(State::new(AppState::Main));
        world.insert_resource(false);
        world.insert_resource("control");
        let mut stage = SystemStage::parallel().with_system(should_run_once);
        stage.run(&mut world);
        assert!(*world.resource::<bool>(), "after control");

        world.insert_resource(false);
        world.insert_resource("test");
        let mut stage = SystemStage::parallel()
            .with_system_set(State::<AppState>::get_driver())
            .with_system(should_run_once);
        stage.run(&mut world);
        assert!(*world.resource::<bool>(), "after test");
    }

    #[test]
    fn restart_state_tests() {
        #[derive(Clone, PartialEq, Eq, Debug, Hash)]
        enum LoadState {
            Load,
            Finish,
        }

        #[derive(PartialEq, Eq, Debug)]
        enum LoadStatus {
            EnterLoad,
            ExitLoad,
            EnterFinish,
        }

        let mut world = World::new();
        world.insert_resource(Vec::<LoadStatus>::new());
        world.insert_resource(State::new(LoadState::Load));

        let mut stage = SystemStage::parallel();
        stage.add_system_set(State::<LoadState>::get_driver());

        // Systems to track loading status
        stage
            .add_system_set(
                State::on_enter_set(LoadState::Load)
                    .with_system(|mut r: ResMut<Vec<LoadStatus>>| r.push(LoadStatus::EnterLoad)),
            )
            .add_system_set(
                State::on_exit_set(LoadState::Load)
                    .with_system(|mut r: ResMut<Vec<LoadStatus>>| r.push(LoadStatus::ExitLoad)),
            )
            .add_system_set(
                State::on_enter_set(LoadState::Finish)
                    .with_system(|mut r: ResMut<Vec<LoadStatus>>| r.push(LoadStatus::EnterFinish)),
            );

        stage.run(&mut world);

        // A. Restart state
        let mut state = world.resource_mut::<State<LoadState>>();
        let result = state.restart();
        assert!(matches!(result, Ok(())));
        stage.run(&mut world);

        // B. Restart state (overwrite schedule)
        let mut state = world.resource_mut::<State<LoadState>>();
        state.set(LoadState::Finish).unwrap();
        state.overwrite_restart();
        stage.run(&mut world);

        // C. Fail restart state (transition already scheduled)
        let mut state = world.resource_mut::<State<LoadState>>();
        state.set(LoadState::Finish).unwrap();
        let result = state.restart();
        assert!(matches!(result, Err(StateError::StateAlreadyQueued)));
        stage.run(&mut world);

        const EXPECTED: &[LoadStatus] = &[
            LoadStatus::EnterLoad,
            // A
            LoadStatus::ExitLoad,
            LoadStatus::EnterLoad,
            // B
            LoadStatus::ExitLoad,
            LoadStatus::EnterLoad,
            // C
            LoadStatus::ExitLoad,
            LoadStatus::EnterFinish,
        ];

        let mut collected = world.resource_mut::<Vec<LoadStatus>>();
        let mut count = 0;
        for (found, expected) in collected.drain(..).zip(EXPECTED) {
            assert_eq!(found, *expected);
            count += 1;
        }
        // If not equal, some elements weren't executed
        assert_eq!(EXPECTED.len(), count);
        assert_eq!(
            world.resource::<State<LoadState>>().current(),
            &LoadState::Finish
        );
    }
}
