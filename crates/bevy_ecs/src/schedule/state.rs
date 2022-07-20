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

/// ### Stack based state machine
///
/// This state machine has four operations: Push, Pop, Set and Replace.
/// * Push pushes a new state to the state stack, pausing the previous state
/// * Pop removes the current state, and unpauses the last paused state
/// * Set replaces the active state with a new one
/// * Replace unwinds the state stack, and replaces the entire stack with a single new state
#[derive(Debug)]
pub struct State<T: StateData> {
    transition: Option<StateTransition<T>>,
    /// The current states in the stack.
    ///
    /// There is always guaranteed to be at least one.
    stack: Vec<T>,
    scheduled: Option<ScheduledOperation<T>>,
    end_next_loop: bool,
}

#[derive(Debug)]
enum StateTransition<T: StateData> {
    PreStartup,
    Startup,
    // The parameter order is always (leaving, entering)
    ExitingToResume(T, T),
    ExitingFull(T, T),
    Entering(T, T),
    Resuming(T, T),
    Pausing(T, T),
}

#[derive(Debug)]
enum ScheduledOperation<T: StateData> {
    Set(T),
    Replace(T),
    Pop,
    Push(T),
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
        (move |state: Res<State<T>>| {
            state.stack.last().unwrap() == &pred && state.transition.is_none()
        })
        .chain(should_run_adapter::<T>)
        .after(DriverLabel::of::<T>())
    }

    pub fn on_inactive_update(pred: T) -> RunCriteriaDescriptor {
        (move |state: Res<State<T>>, mut is_inactive: Local<bool>| match &state.transition {
            Some(StateTransition::Pausing(ref relevant, _))
            | Some(StateTransition::Resuming(_, ref relevant)) => {
                if relevant == &pred {
                    *is_inactive = !*is_inactive;
                }
                false
            }
            Some(_) => false,
            None => *is_inactive,
        })
        .chain(should_run_adapter::<T>)
        .after(DriverLabel::of::<T>())
    }

    pub fn on_in_stack_update(pred: T) -> RunCriteriaDescriptor {
        (move |state: Res<State<T>>, mut is_in_stack: Local<bool>| match &state.transition {
            Some(StateTransition::Entering(ref relevant, _))
            | Some(StateTransition::ExitingToResume(_, ref relevant))
            | Some(StateTransition::ExitingFull(_, ref relevant)) => {
                if relevant == &pred {
                    *is_in_stack = !*is_in_stack;
                }
                false
            }
            Some(StateTransition::Startup) => {
                if state.stack.last().unwrap() == &pred {
                    *is_in_stack = !*is_in_stack;
                }
                false
            }
            Some(_) => false,
            None => *is_in_stack,
        })
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
                    StateTransition::Startup => state.stack.last().unwrap() == &pred,
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
                    StateTransition::ExitingToResume(exiting, _)
                    | StateTransition::ExitingFull(exiting, _) => exiting == &pred,
                    _ => false,
                })
        })
        .chain(should_run_adapter::<T>)
        .after(DriverLabel::of::<T>())
    }

    pub fn on_pause(pred: T) -> RunCriteriaDescriptor {
        (move |state: Res<State<T>>| {
            state
                .transition
                .as_ref()
                .map_or(false, |transition| match transition {
                    StateTransition::Pausing(pausing, _) => pausing == &pred,
                    _ => false,
                })
        })
        .chain(should_run_adapter::<T>)
        .after(DriverLabel::of::<T>())
    }

    pub fn on_resume(pred: T) -> RunCriteriaDescriptor {
        (move |state: Res<State<T>>| {
            state
                .transition
                .as_ref()
                .map_or(false, |transition| match transition {
                    StateTransition::Resuming(_, resuming) => resuming == &pred,
                    _ => false,
                })
        })
        .chain(should_run_adapter::<T>)
        .after(DriverLabel::of::<T>())
    }

    pub fn on_update_set(s: T) -> SystemSet {
        SystemSet::new().with_run_criteria(Self::on_update(s))
    }

    pub fn on_inactive_update_set(s: T) -> SystemSet {
        SystemSet::new().with_run_criteria(Self::on_inactive_update(s))
    }

    pub fn on_enter_set(s: T) -> SystemSet {
        SystemSet::new().with_run_criteria(Self::on_enter(s))
    }

    pub fn on_exit_set(s: T) -> SystemSet {
        SystemSet::new().with_run_criteria(Self::on_exit(s))
    }

    pub fn on_pause_set(s: T) -> SystemSet {
        SystemSet::new().with_run_criteria(Self::on_pause(s))
    }

    pub fn on_resume_set(s: T) -> SystemSet {
        SystemSet::new().with_run_criteria(Self::on_resume(s))
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
            stack: vec![initial],
            transition: Some(StateTransition::PreStartup),
            scheduled: None,
            end_next_loop: false,
        }
    }

    /// Schedule a state change that replaces the active state with the given state.
    /// This will fail if there is a scheduled operation, pending transition, or if the given
    /// `state` matches the current state
    pub fn set(&mut self, state: T) -> Result<(), StateError> {
        if self.stack.last().unwrap() == &state {
            return Err(StateError::AlreadyInState);
        }

        if self.scheduled.is_some() || self.transition.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        self.scheduled = Some(ScheduledOperation::Set(state));
        Ok(())
    }

    /// Same as [`Self::set`], but if there is already a next state, it will be overwritten
    /// instead of failing
    pub fn overwrite_set(&mut self, state: T) -> Result<(), StateError> {
        if self.stack.last().unwrap() == &state {
            return Err(StateError::AlreadyInState);
        }

        self.scheduled = Some(ScheduledOperation::Set(state));
        Ok(())
    }

    /// Schedule a state change that replaces the full stack with the given state.
    /// This will fail if there is a scheduled operation, pending transition, or if the given
    /// `state` matches the current state
    pub fn replace(&mut self, state: T) -> Result<(), StateError> {
        if self.stack.last().unwrap() == &state {
            return Err(StateError::AlreadyInState);
        }

        if self.scheduled.is_some() || self.transition.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        self.scheduled = Some(ScheduledOperation::Replace(state));
        Ok(())
    }

    /// Same as [`Self::replace`], but if there is already a next state, it will be overwritten
    /// instead of failing
    pub fn overwrite_replace(&mut self, state: T) -> Result<(), StateError> {
        if self.stack.last().unwrap() == &state {
            return Err(StateError::AlreadyInState);
        }

        self.scheduled = Some(ScheduledOperation::Replace(state));
        Ok(())
    }

    /// Same as [`Self::set`], but does a push operation instead of a next operation
    pub fn push(&mut self, state: T) -> Result<(), StateError> {
        if self.stack.last().unwrap() == &state {
            return Err(StateError::AlreadyInState);
        }

        if self.scheduled.is_some() || self.transition.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        self.scheduled = Some(ScheduledOperation::Push(state));
        Ok(())
    }

    /// Same as [`Self::push`], but if there is already a next state, it will be overwritten
    /// instead of failing
    pub fn overwrite_push(&mut self, state: T) -> Result<(), StateError> {
        if self.stack.last().unwrap() == &state {
            return Err(StateError::AlreadyInState);
        }

        self.scheduled = Some(ScheduledOperation::Push(state));
        Ok(())
    }

    /// Same as [`Self::set`], but does a pop operation instead of a set operation
    pub fn pop(&mut self) -> Result<(), StateError> {
        if self.scheduled.is_some() || self.transition.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        if self.stack.len() == 1 {
            return Err(StateError::StackEmpty);
        }

        self.scheduled = Some(ScheduledOperation::Pop);
        Ok(())
    }

    /// Same as [`Self::pop`], but if there is already a next state, it will be overwritten
    /// instead of failing
    pub fn overwrite_pop(&mut self) -> Result<(), StateError> {
        if self.stack.len() == 1 {
            return Err(StateError::StackEmpty);
        }
        self.scheduled = Some(ScheduledOperation::Pop);
        Ok(())
    }

    /// Schedule a state change that restarts the active state.
    /// This will fail if there is a scheduled operation or a pending transition
    pub fn restart(&mut self) -> Result<(), StateError> {
        if self.scheduled.is_some() || self.transition.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        let state = self.stack.last().unwrap();
        self.scheduled = Some(ScheduledOperation::Set(state.clone()));
        Ok(())
    }

    /// Same as [`Self::restart`], but if there is already a scheduled state operation,
    /// it will be overwritten instead of failing
    pub fn overwrite_restart(&mut self) {
        let state = self.stack.last().unwrap();
        self.scheduled = Some(ScheduledOperation::Set(state.clone()));
    }

    pub fn current(&self) -> &T {
        self.stack.last().unwrap()
    }

    pub fn inactives(&self) -> &[T] {
        self.stack.split_last().map(|(_, rest)| rest).unwrap()
    }

    /// Clears the scheduled state operation.
    pub fn clear_schedule(&mut self) {
        self.scheduled = None;
    }
}

#[derive(Debug)]
pub enum StateError {
    AlreadyInState,
    StateAlreadyQueued,
    StackEmpty,
}

impl std::error::Error for StateError {}

impl fmt::Display for StateError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            StateError::AlreadyInState => {
                write!(f, "Attempted to change the state to the current state.")
            }
            StateError::StateAlreadyQueued => write!(
                f,
                "Attempted to queue a state change, but there was already a state queued."
            ),
            StateError::StackEmpty => {
                write!(f, "Attempted to queue a pop, but there is nothing to pop.")
            }
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
        if state.scheduled.is_none() {
            state.end_next_loop = true;
            return ShouldRun::YesAndCheckAgain;
        }
    } else if state.end_next_loop {
        state.end_next_loop = false;
        return ShouldRun::No;
    }
    match state.scheduled.take() {
        Some(ScheduledOperation::Set(next)) => {
            state.transition = Some(StateTransition::ExitingFull(
                state.stack.last().unwrap().clone(),
                next,
            ));
        }
        Some(ScheduledOperation::Replace(next)) => {
            if state.stack.len() <= 1 {
                state.transition = Some(StateTransition::ExitingFull(
                    state.stack.last().unwrap().clone(),
                    next,
                ));
            } else {
                state.scheduled = Some(ScheduledOperation::Replace(next));
                match state.transition.take() {
                    Some(StateTransition::ExitingToResume(p, n)) => {
                        state.stack.pop();
                        state.transition = Some(StateTransition::Resuming(p, n));
                    }
                    _ => {
                        state.transition = Some(StateTransition::ExitingToResume(
                            state.stack[state.stack.len() - 1].clone(),
                            state.stack[state.stack.len() - 2].clone(),
                        ));
                    }
                }
            }
        }
        Some(ScheduledOperation::Push(next)) => {
            let last_type_id = state.stack.last().unwrap().clone();
            state.transition = Some(StateTransition::Pausing(last_type_id, next));
        }
        Some(ScheduledOperation::Pop) => {
            state.transition = Some(StateTransition::ExitingToResume(
                state.stack[state.stack.len() - 1].clone(),
                state.stack[state.stack.len() - 2].clone(),
            ));
        }
        None => match state.transition.take() {
            Some(StateTransition::ExitingFull(p, n)) => {
                state.transition = Some(StateTransition::Entering(p, n.clone()));
                *state.stack.last_mut().unwrap() = n;
            }
            Some(StateTransition::Pausing(p, n)) => {
                state.transition = Some(StateTransition::Entering(p, n.clone()));
                state.stack.push(n);
            }
            Some(StateTransition::ExitingToResume(p, n)) => {
                state.stack.pop();
                state.transition = Some(StateTransition::Resuming(p, n));
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
                    s.overwrite_replace(MyState::S2).unwrap();
                },
            ))
            .add_system_set(
                State::on_enter_set(MyState::S2)
                    .with_system(|mut r: ResMut<Vec<&'static str>>| r.push("enter S2")),
            )
            .add_system_set(State::on_update_set(MyState::S2).with_system(
                |mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                    r.push("update S2");
                    s.overwrite_replace(MyState::S3).unwrap();
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
                    s.overwrite_push(MyState::S4).unwrap();
                },
            ))
            .add_system_set(
                State::on_pause_set(MyState::S3)
                    .with_system(|mut r: ResMut<Vec<&'static str>>| r.push("pause S3")),
            )
            .add_system_set(State::on_update_set(MyState::S4).with_system(
                |mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                    r.push("update S4");
                    s.overwrite_push(MyState::S5).unwrap();
                },
            ))
            .add_system_set(State::on_inactive_update_set(MyState::S4).with_system(
                (|mut r: ResMut<Vec<&'static str>>| r.push("inactive S4")).label("inactive s4"),
            ))
            .add_system_set(
                State::on_update_set(MyState::S5).with_system(
                    (|mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                        r.push("update S5");
                        s.overwrite_push(MyState::S6).unwrap();
                    })
                    .after("inactive s4"),
                ),
            )
            .add_system_set(
                State::on_inactive_update_set(MyState::S5).with_system(
                    (|mut r: ResMut<Vec<&'static str>>| r.push("inactive S5"))
                        .label("inactive s5")
                        .after("inactive s4"),
                ),
            )
            .add_system_set(
                State::on_update_set(MyState::S6).with_system(
                    (|mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                        r.push("update S6");
                        s.overwrite_push(MyState::Final).unwrap();
                    })
                    .after("inactive s5"),
                ),
            )
            .add_system_set(
                State::on_resume_set(MyState::S4)
                    .with_system(|mut r: ResMut<Vec<&'static str>>| r.push("resume S4")),
            )
            .add_system_set(
                State::on_exit_set(MyState::S5)
                    .with_system(|mut r: ResMut<Vec<&'static str>>| r.push("exit S4")),
            );

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
            "pause S3",
            "update S4",
            //
            "inactive S4",
            "update S5",
            //
            "inactive S4",
            "inactive S5",
            "update S6",
            //
            "inactive S4",
            "inactive S5",
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
