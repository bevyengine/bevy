#![allow(clippy::clippy::mem_discriminant_non_enum)]

use std::{
    any::TypeId,
    marker::PhantomData,
    mem::{discriminant, Discriminant},
};

use bevy_utils::HashMap;

use crate::{
    ArchetypeComponent, IntoSystem, ResMut, Resource, ShouldRun, System, SystemDescriptor,
    SystemId, SystemSet, SystemStage, TypeAccess,
};
#[derive(Debug)]
pub struct SetState<T: Clone> {
    transition: Option<StateTransition<T>>,
    stack: Vec<T>,
    scheduled: Option<ScheduledOperation<T>>,
}

#[derive(Debug)]
enum StateTransition<T: Clone> {
    ExitingToResume(T, T),
    ExitingFull(T, T),
    Entering(T, T),
    Resuming(T, T),
    Pausing(T, T),
}

#[derive(Debug)]
pub enum ScheduledOperation<T: Clone> {
    Next(T),
    Pop,
    Push(T),
}

impl<T: Clone + Resource> SetState<T> {
    fn on_update(d: Discriminant<T>) -> impl System<In = (), Out = ShouldRun> {
        Wrapper::<T, OnUpdate>::new(d)
    }

    fn on_enter(d: Discriminant<T>) -> impl System<In = (), Out = ShouldRun> {
        Wrapper::<T, OnEnter>::new(d)
    }

    fn on_exit(d: Discriminant<T>) -> impl System<In = (), Out = ShouldRun> {
        Wrapper::<T, OnExit>::new(d)
    }

    fn on_pause(d: Discriminant<T>) -> impl System<In = (), Out = ShouldRun> {
        Wrapper::<T, OnExit>::new(d)
    }

    fn on_resume(d: Discriminant<T>) -> impl System<In = (), Out = ShouldRun> {
        Wrapper::<T, OnExit>::new(d)
    }

    pub fn schedule_operation(
        &mut self,
        val: ScheduledOperation<T>,
    ) -> Option<ScheduledOperation<T>> {
        self.scheduled.replace(val)
    }

    pub fn new(val: T) -> Self {
        Self {
            stack: vec![val],
            transition: None,
            scheduled: None,
        }
    }

    pub fn current(&self) -> &T {
        self.stack.last().unwrap()
    }
}

trait Comparer<T: Clone> {
    fn compare(d: Discriminant<T>, s: &SetState<T>) -> bool;
}

struct OnUpdate;
impl<T: Clone> Comparer<T> for OnUpdate {
    fn compare(d: Discriminant<T>, s: &SetState<T>) -> bool {
        discriminant(s.stack.last().unwrap()) == d && s.transition.is_none()
    }
}
struct OnEnter;
impl<T: Clone> Comparer<T> for OnEnter {
    fn compare(d: Discriminant<T>, s: &SetState<T>) -> bool {
        s.transition
            .as_ref()
            .map_or(false, |transition| match transition {
                StateTransition::Entering(_, entering) => discriminant(entering) == d,
                _ => false,
            })
    }
}
struct OnExit;
impl<T: Clone> Comparer<T> for OnExit {
    fn compare(d: Discriminant<T>, s: &SetState<T>) -> bool {
        s.transition
            .as_ref()
            .map_or(false, |transition| match transition {
                StateTransition::ExitingToResume(exiting, _)
                | StateTransition::ExitingFull(exiting, _) => discriminant(exiting) == d,
                _ => false,
            })
    }
}
struct OnPause;
impl<T: Clone> Comparer<T> for OnPause {
    fn compare(d: Discriminant<T>, s: &SetState<T>) -> bool {
        s.transition
            .as_ref()
            .map_or(false, |transition| match transition {
                StateTransition::Pausing(pausing, _) => discriminant(pausing) == d,
                _ => false,
            })
    }
}
struct OnResume;
impl<T: Clone> Comparer<T> for OnResume {
    fn compare(d: Discriminant<T>, s: &SetState<T>) -> bool {
        s.transition
            .as_ref()
            .map_or(false, |transition| match transition {
                StateTransition::Resuming(_, resuming) => discriminant(resuming) == d,
                _ => false,
            })
    }
}

impl<T: Clone + Resource, C: Comparer<T>> Wrapper<T, C> {
    fn new(discriminant: Discriminant<T>) -> Self {
        let mut resource_access = TypeAccess::default();
        resource_access.add_read(std::any::TypeId::of::<SetState<T>>());
        Self {
            discriminant,
            exit_flag: false,
            resource_access,
            id: SystemId::new(),
            archetype_access: Default::default(),
            component_access: Default::default(),
            marker: Default::default(),
        }
    }
}

struct Wrapper<T: Clone + Resource, C: Comparer<T>> {
    discriminant: Discriminant<T>,
    exit_flag: bool,
    resource_access: TypeAccess<TypeId>,
    id: SystemId,
    archetype_access: TypeAccess<ArchetypeComponent>,
    component_access: TypeAccess<TypeId>,
    marker: PhantomData<C>,
}

impl<T: Clone + Resource, C: Comparer<T> + Resource> System for Wrapper<T, C> {
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
        let state = &*resources.get::<SetState<T>>().unwrap();
        if state.transition.is_some() {
            self.exit_flag = false;
        }
        if self.exit_flag {
            self.exit_flag = false;
            Some(ShouldRun::No)
        } else {
            self.exit_flag = true;
            Some(if C::compare(self.discriminant, state) {
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

pub struct StateSetBuilder<T: Clone + Resource> {
    on_update: HashMap<Discriminant<T>, SystemSet>,
    on_enter: HashMap<Discriminant<T>, SystemSet>,
    on_exit: HashMap<Discriminant<T>, SystemSet>,
    on_pause: HashMap<Discriminant<T>, SystemSet>,
    on_resume: HashMap<Discriminant<T>, SystemSet>,
}

impl<T: Clone + Resource> Default for StateSetBuilder<T> {
    fn default() -> Self {
        Self {
            on_update: Default::default(),
            on_enter: Default::default(),
            on_exit: Default::default(),
            on_pause: Default::default(),
            on_resume: Default::default(),
        }
    }
}

impl<T: Clone + Resource> StateSetBuilder<T> {
    pub fn add_on_update(&mut self, v: T, system: impl Into<SystemDescriptor>) -> &mut Self {
        self.on_update
            .entry(discriminant(&v))
            .or_default()
            .add_system(system);
        self
    }

    pub fn add_on_enter(&mut self, v: T, system: impl Into<SystemDescriptor>) -> &mut Self {
        self.on_enter
            .entry(discriminant(&v))
            .or_default()
            .add_system(system);
        self
    }

    pub fn add_on_exit(&mut self, v: T, system: impl Into<SystemDescriptor>) -> &mut Self {
        self.on_exit
            .entry(discriminant(&v))
            .or_default()
            .add_system(system);
        self
    }

    pub fn add_on_pause(&mut self, v: T, system: impl Into<SystemDescriptor>) -> &mut Self {
        self.on_pause
            .entry(discriminant(&v))
            .or_default()
            .add_system(system);
        self
    }

    pub fn add_on_resume(&mut self, v: T, system: impl Into<SystemDescriptor>) -> &mut Self {
        self.on_resume
            .entry(discriminant(&v))
            .or_default()
            .add_system(system);
        self
    }

    pub fn with_on_update(mut self, v: T, system: impl Into<SystemDescriptor>) -> Self {
        self.add_on_update(v, system);
        self
    }

    pub fn with_on_enter(mut self, v: T, system: impl Into<SystemDescriptor>) -> Self {
        self.add_on_enter(v, system);
        self
    }

    pub fn with_on_exit(mut self, v: T, system: impl Into<SystemDescriptor>) -> Self {
        self.add_on_exit(v, system);
        self
    }

    pub fn with_on_pause(mut self, v: T, system: impl Into<SystemDescriptor>) -> Self {
        self.add_on_pause(v, system);
        self
    }

    pub fn with_on_resume(mut self, v: T, system: impl Into<SystemDescriptor>) -> Self {
        self.add_on_resume(v, system);
        self
    }

    pub fn finalize(self, stage: &mut SystemStage) {
        fn state_cleaner<T: Clone + Resource>(mut state: ResMut<SetState<T>>) -> ShouldRun {
            match state.scheduled.take() {
                Some(ScheduledOperation::Next(next)) => {
                    if state.stack.len() == 1 {
                        let previous =
                            std::mem::replace(state.stack.last_mut().unwrap(), next.clone());
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
                    Some(StateTransition::ExitingFull(p, n))
                    | Some(StateTransition::Pausing(p, n)) => {
                        state.transition = Some(StateTransition::Entering(p, n));
                    }
                    Some(StateTransition::ExitingToResume(p, n)) => {
                        state.transition = Some(StateTransition::Resuming(p, n));
                    }
                    _ => return ShouldRun::Yes,
                },
            };
            ShouldRun::YesAndCheckAgain
        }

        for (val, set) in self.on_enter.into_iter() {
            stage.add_system_set(set.with_run_criteria(SetState::<T>::on_enter(val)));
        }
        for (val, set) in self.on_update.into_iter() {
            stage.add_system_set(set.with_run_criteria(SetState::<T>::on_update(val)));
        }
        for (val, set) in self.on_exit.into_iter() {
            stage.add_system_set(set.with_run_criteria(SetState::<T>::on_exit(val)));
        }
        for (val, set) in self.on_pause.into_iter() {
            stage.add_system_set(set.with_run_criteria(SetState::<T>::on_pause(val)));
        }
        for (val, set) in self.on_resume.into_iter() {
            stage.add_system_set(set.with_run_criteria(SetState::<T>::on_resume(val)));
        }

        stage.add_system_set(SystemSet::default().with_run_criteria(state_cleaner::<T>.system()));
    }
}
