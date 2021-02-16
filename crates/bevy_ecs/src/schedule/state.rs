#![allow(clippy::clippy::mem_discriminant_non_enum)]

use std::{
    any::TypeId,
    mem::{discriminant, Discriminant},
};

use crate::{
    ArchetypeComponent, IntoSystem, ResMut, Resource, ShouldRun, System, SystemId, SystemSet,
    TypeAccess,
};
use thiserror::Error;

/// ### Stack based state machine
/// Important: `T` must be an enum!
///
/// This state machine has three operations: Next, Pop, and Push.
/// * Push pushes a new state to the state stack, pausing the previous state
/// * Pop removes the current state, and unpauses the last paused state.
/// * Next unwinds the state stack, and replaces the entire stack with a single new state
#[derive(Debug)]
pub struct State<T: Clone> {
    transition: Option<StateTransition<T>>,
    stack: Vec<T>,
    scheduled: Option<ScheduledOperation<T>>,
}

#[derive(Debug)]
enum StateTransition<T: Clone> {
    PreStartup,
    Startup,
    ExitingToResume(T, T),
    ExitingFull(T, T),
    Entering(T, T),
    Resuming(T, T),
    Pausing(T, T),
}

#[derive(Debug)]
enum ScheduledOperation<T: Clone> {
    Next(T),
    Pop,
    Push(T),
}

impl<T: Clone + Resource> State<T> {
    pub fn on_update(d: T) -> impl System<In = (), Out = ShouldRun> {
        Wrapper::<T, OnUpdate>::new(discriminant(&d))
    }

    pub fn on_inactive_update(d: T) -> impl System<In = (), Out = ShouldRun> {
        Wrapper::<T, OnInactiveUpdate>::new(discriminant(&d))
    }

    pub fn on_enter(d: T) -> impl System<In = (), Out = ShouldRun> {
        Wrapper::<T, OnEnter>::new(discriminant(&d))
    }

    pub fn on_exit(d: T) -> impl System<In = (), Out = ShouldRun> {
        Wrapper::<T, OnExit>::new(discriminant(&d))
    }

    pub fn on_pause(d: T) -> impl System<In = (), Out = ShouldRun> {
        Wrapper::<T, OnExit>::new(discriminant(&d))
    }

    pub fn on_resume(d: T) -> impl System<In = (), Out = ShouldRun> {
        Wrapper::<T, OnExit>::new(discriminant(&d))
    }

    /// Creates a driver set for the State.
    ///
    /// Important note: this set must be inserted **before** all other state-dependant sets to work properly!
    pub fn make_driver() -> SystemSet {
        SystemSet::default().with_run_criteria(state_cleaner::<T>.system())
    }

    pub fn new(val: T) -> Self {
        Self {
            stack: vec![val],
            transition: Some(StateTransition::PreStartup),
            scheduled: None,
        }
    }

    /// Schedule a state change that replaces the full stack with the given state.
    /// This will fail if there is a scheduled operation, or if the given `state` matches the current state
    pub fn set_next(&mut self, state: T) -> Result<(), StateError> {
        if discriminant(self.stack.last().unwrap()) == discriminant(&state) {
            return Err(StateError::AlreadyInState);
        }

        if self.scheduled.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        self.scheduled = Some(ScheduledOperation::Next(state));
        Ok(())
    }

    /// Same as [Self::set_next], but if there is already a next state, it will be overwritten instead of failing
    pub fn overwrite_next(&mut self, state: T) -> Result<(), StateError> {
        if discriminant(self.stack.last().unwrap()) == discriminant(&state) {
            return Err(StateError::AlreadyInState);
        }

        self.scheduled = Some(ScheduledOperation::Next(state));
        Ok(())
    }

    /// Same as [Self::set_next], but does a push operation instead of a next operation
    pub fn set_push(&mut self, state: T) -> Result<(), StateError> {
        if discriminant(self.stack.last().unwrap()) == discriminant(&state) {
            return Err(StateError::AlreadyInState);
        }

        if self.scheduled.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        self.scheduled = Some(ScheduledOperation::Push(state));
        Ok(())
    }

    /// Same as [Self::set_push], but if there is already a next state, it will be overwritten instead of failing
    pub fn overwrite_push(&mut self, state: T) -> Result<(), StateError> {
        if discriminant(self.stack.last().unwrap()) == discriminant(&state) {
            return Err(StateError::AlreadyInState);
        }

        self.scheduled = Some(ScheduledOperation::Push(state));
        Ok(())
    }

    /// Same as [Self::set_next], but does a pop operation instead of a next operation
    pub fn set_pop(&mut self) -> Result<(), StateError> {
        if self.scheduled.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        if self.stack.len() == 1 {
            return Err(StateError::StackEmpty);
        }

        self.scheduled = Some(ScheduledOperation::Pop);
        Ok(())
    }

    /// Same as [Self::set_pop], but if there is already a next state, it will be overwritten instead of failing
    pub fn overwrite_pop(&mut self) -> Result<(), StateError> {
        if self.stack.len() == 1 {
            return Err(StateError::StackEmpty);
        }
        self.scheduled = Some(ScheduledOperation::Pop);
        Ok(())
    }

    pub fn current(&self) -> &T {
        self.stack.last().unwrap()
    }

    pub fn inactives(&self) -> &[T] {
        &self.stack[0..self.stack.len() - 1]
    }
}

#[derive(Debug, Error)]
pub enum StateError {
    #[error("Attempted to change the state to the current state.")]
    AlreadyInState,
    #[error("Attempted to queue a state change, but there was already a state queued.")]
    StateAlreadyQueued,
    #[error("Attempted to queue a pop, but there is nothing to pop.")]
    StackEmpty,
}

trait Comparer<T: Clone> {
    fn compare(&mut self, d: Discriminant<T>, s: &State<T>) -> bool;
}

#[derive(Default)]
struct OnUpdate;
impl<T: Clone> Comparer<T> for OnUpdate {
    fn compare(&mut self, d: Discriminant<T>, s: &State<T>) -> bool {
        discriminant(s.stack.last().unwrap()) == d && s.transition.is_none()
    }
}
#[derive(Default)]
struct OnInactiveUpdate(bool /*tracks if the relevant state has been entered*/);
impl<T: Clone> Comparer<T> for OnInactiveUpdate {
    fn compare(&mut self, d: Discriminant<T>, s: &State<T>) -> bool {
        match &s.transition {
            Some(StateTransition::Pausing(ref relevant, _))
            | Some(StateTransition::Resuming(_, ref relevant)) => {
                if discriminant(relevant) == d {
                    self.0 = !self.0;
                }
                false
            }
            Some(_) => false,
            None => self.0,
        }
    }
}
#[derive(Default)]
struct OnEnter;
impl<T: Clone> Comparer<T> for OnEnter {
    fn compare(&mut self, d: Discriminant<T>, s: &State<T>) -> bool {
        s.transition
            .as_ref()
            .map_or(false, |transition| match transition {
                StateTransition::Entering(_, entering) => discriminant(entering) == d,
                StateTransition::Startup => discriminant(s.stack.last().unwrap()) == d,
                _ => false,
            })
    }
}
#[derive(Default)]
struct OnExit;
impl<T: Clone> Comparer<T> for OnExit {
    fn compare(&mut self, d: Discriminant<T>, s: &State<T>) -> bool {
        s.transition
            .as_ref()
            .map_or(false, |transition| match transition {
                StateTransition::ExitingToResume(exiting, _)
                | StateTransition::ExitingFull(exiting, _) => discriminant(exiting) == d,
                _ => false,
            })
    }
}
#[derive(Default)]
struct OnPause;
impl<T: Clone> Comparer<T> for OnPause {
    fn compare(&mut self, d: Discriminant<T>, s: &State<T>) -> bool {
        s.transition
            .as_ref()
            .map_or(false, |transition| match transition {
                StateTransition::Pausing(pausing, _) => discriminant(pausing) == d,
                _ => false,
            })
    }
}
#[derive(Default)]
struct OnResume;
impl<T: Clone> Comparer<T> for OnResume {
    fn compare(&mut self, d: Discriminant<T>, s: &State<T>) -> bool {
        s.transition
            .as_ref()
            .map_or(false, |transition| match transition {
                StateTransition::Resuming(_, resuming) => discriminant(resuming) == d,
                _ => false,
            })
    }
}

impl<T: Clone + Resource, C: Comparer<T> + Default> Wrapper<T, C> {
    fn new(discriminant: Discriminant<T>) -> Self {
        let mut resource_access = TypeAccess::default();
        resource_access.add_read(std::any::TypeId::of::<State<T>>());
        Self {
            discriminant,
            exit_flag: false,
            resource_access,
            id: SystemId::new(),
            archetype_access: Default::default(),
            component_access: Default::default(),
            comparer: Default::default(),
        }
    }
}

struct Wrapper<T: Clone + Resource, C: Comparer<T> + Default> {
    discriminant: Discriminant<T>,
    exit_flag: bool,
    resource_access: TypeAccess<TypeId>,
    id: SystemId,
    archetype_access: TypeAccess<ArchetypeComponent>,
    component_access: TypeAccess<TypeId>,
    comparer: C,
}

impl<T: Clone + Resource, C: Comparer<T> + Resource + Default> System for Wrapper<T, C> {
    type In = ();
    type Out = ShouldRun;

    fn name(&self) -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Owned(format!(
            "State checker for state {}",
            std::any::type_name::<T>()
        ))
    }

    fn id(&self) -> crate::SystemId {
        self.id
    }

    fn archetype_component_access(&self) -> &TypeAccess<ArchetypeComponent> {
        &self.archetype_access
    }

    fn resource_access(&self) -> &TypeAccess<TypeId> {
        &self.resource_access
    }

    fn component_access(&self) -> &TypeAccess<TypeId> {
        &self.component_access
    }

    fn is_non_send(&self) -> bool {
        false
    }

    unsafe fn run_unsafe(
        &mut self,
        _input: Self::In,
        _world: &crate::World,
        resources: &crate::Resources,
    ) -> Option<Self::Out> {
        let state = &*resources.get::<State<T>>().unwrap();
        if state.transition.is_some() {
            self.exit_flag = false;
        }
        if self.exit_flag {
            self.exit_flag = false;
            Some(ShouldRun::No)
        } else {
            self.exit_flag = true;
            Some(if self.comparer.compare(self.discriminant, state) {
                ShouldRun::YesAndCheckAgain
            } else {
                ShouldRun::NoAndCheckAgain
            })
        }
    }

    fn update_access(&mut self, _world: &crate::World) {}

    fn apply_buffers(&mut self, _world: &mut crate::World, _resources: &mut crate::Resources) {}

    fn initialize(&mut self, _world: &mut crate::World, _resources: &mut crate::Resources) {}
}

fn state_cleaner<T: Clone + Resource>(mut state: ResMut<State<T>>) -> ShouldRun {
    match state.scheduled.take() {
        Some(ScheduledOperation::Next(next)) => {
            if state.stack.len() <= 1 {
                let previous = std::mem::replace(state.stack.last_mut().unwrap(), next.clone());
                state.transition = Some(StateTransition::ExitingFull(previous, next));
            } else {
                state.scheduled = Some(ScheduledOperation::Next(next));
                match state.transition.take() {
                    Some(StateTransition::ExitingToResume(p, n)) => {
                        state.transition = Some(StateTransition::Resuming(p, n));
                    }
                    _ => {
                        state.transition = Some(StateTransition::ExitingToResume(
                            state.stack.pop().unwrap(),
                            state.stack.last().unwrap().clone(),
                        ));
                    }
                }
            }
        }
        Some(ScheduledOperation::Push(next)) => {
            let last = state.stack.last().unwrap().clone();
            state.stack.push(next.clone());
            state.transition = Some(StateTransition::Pausing(last, next));
        }
        Some(ScheduledOperation::Pop) => {
            state.transition = Some(StateTransition::ExitingToResume(
                state.stack.pop().unwrap(),
                state.stack.last().unwrap().clone(),
            ));
        }
        None => match state.transition.take() {
            Some(StateTransition::ExitingFull(p, n)) | Some(StateTransition::Pausing(p, n)) => {
                state.transition = Some(StateTransition::Entering(p, n));
            }
            Some(StateTransition::ExitingToResume(p, n)) => {
                state.transition = Some(StateTransition::Resuming(p, n));
            }
            Some(StateTransition::PreStartup) => {
                state.transition = Some(StateTransition::Startup);
            }
            _ => return ShouldRun::Yes,
        },
    };
    ShouldRun::YesAndCheckAgain
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    #[derive(Clone, Copy, PartialEq, Eq)]
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
        let mut resources = Resources::default();

        resources.insert(Vec::<&'static str>::new());
        resources.insert(State::new(MyState::S1));

        let mut stage = SystemStage::parallel();

        stage.add_system_set(State::<MyState>::make_driver());
        stage
            .add_system_set(
                SystemSet::default()
                    .with_system(
                        (|mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                            r.push("update S1");
                            s.overwrite_next(MyState::S2).unwrap();
                        })
                        .system(),
                    )
                    .with_run_criteria(State::<MyState>::on_update(MyState::S1)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system((|mut r: ResMut<Vec<&'static str>>| r.push("enter S2")).system())
                    .with_run_criteria(State::<MyState>::on_enter(MyState::S2)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system(
                        (|mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                            r.push("update S2");
                            s.overwrite_next(MyState::S3).unwrap();
                        })
                        .system(),
                    )
                    .with_run_criteria(State::<MyState>::on_update(MyState::S2)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system((|mut r: ResMut<Vec<&'static str>>| r.push("exit S2")).system())
                    .with_run_criteria(State::<MyState>::on_exit(MyState::S2)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system((|mut r: ResMut<Vec<&'static str>>| r.push("enter S3")).system())
                    .with_run_criteria(State::<MyState>::on_enter(MyState::S3)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system(
                        (|mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                            r.push("update S3");
                            s.overwrite_push(MyState::S4).unwrap();
                        })
                        .system(),
                    )
                    .with_run_criteria(State::<MyState>::on_update(MyState::S3)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system((|mut r: ResMut<Vec<&'static str>>| r.push("pause S3")).system())
                    .with_run_criteria(State::<MyState>::on_pause(MyState::S3)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system(
                        (|mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                            r.push("update S4");
                            s.overwrite_push(MyState::S5).unwrap();
                        })
                        .system(),
                    )
                    .with_run_criteria(State::<MyState>::on_update(MyState::S4)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system(
                        (|mut r: ResMut<Vec<&'static str>>| r.push("inactive S4")).system(),
                    )
                    .with_run_criteria(State::<MyState>::on_inactive_update(MyState::S4)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system(
                        (|mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                            r.push("update S5");
                            s.overwrite_push(MyState::S6).unwrap();
                        })
                        .system(),
                    )
                    .with_run_criteria(State::<MyState>::on_update(MyState::S5)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system(
                        (|mut r: ResMut<Vec<&'static str>>| r.push("inactive S5")).system(),
                    )
                    .with_run_criteria(State::<MyState>::on_inactive_update(MyState::S5)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system(
                        (|mut r: ResMut<Vec<&'static str>>, mut s: ResMut<State<MyState>>| {
                            r.push("update S6");
                            s.overwrite_push(MyState::Final).unwrap();
                        })
                        .system(),
                    )
                    .with_run_criteria(State::<MyState>::on_update(MyState::S6)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system((|mut r: ResMut<Vec<&'static str>>| r.push("resume S4")).system())
                    .with_run_criteria(State::<MyState>::on_resume(MyState::S4)),
            )
            .add_system_set(
                SystemSet::default()
                    .with_system((|mut r: ResMut<Vec<&'static str>>| r.push("exit S4")).system())
                    .with_run_criteria(State::<MyState>::on_exit(MyState::S4)),
            );

        loop {
            println!("new run!");
            stage.run(&mut world, &mut resources);
            for s in resources.get_mut::<Vec<&'static str>>().unwrap().drain(..) {
                println!("{}", s);
            }

            if resources.get::<State<MyState>>().unwrap().current() == &MyState::Final {
                break;
            }
        }
    }
}
