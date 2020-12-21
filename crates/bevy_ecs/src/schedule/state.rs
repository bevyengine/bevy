use crate::{Resource, Resources, Stage, System, SystemStage, World};
use bevy_utils::HashMap;
use std::{
    mem::{self, Discriminant},
    ops::Deref,
};
use thiserror::Error;

pub(crate) struct StateStages {
    update: Box<dyn Stage>,
    enter: Box<dyn Stage>,
    exit: Box<dyn Stage>,
}

impl Default for StateStages {
    fn default() -> Self {
        Self {
            enter: Box::new(SystemStage::parallel()),
            update: Box::new(SystemStage::parallel()),
            exit: Box::new(SystemStage::parallel()),
        }
    }
}

pub struct StateStage<T> {
    stages: HashMap<Discriminant<T>, StateStages>,
    current_stage: Option<Discriminant<T>>,
}

impl<T> Default for StateStage<T> {
    fn default() -> Self {
        Self {
            stages: Default::default(),
            current_stage: None,
        }
    }
}

#[allow(clippy::mem_discriminant_non_enum)]
impl<T> StateStage<T> {
    pub fn with_enter_stage<S: Stage>(mut self, state: T, stage: S) -> Self {
        self.set_enter_stage(state, stage);
        self
    }

    pub fn with_exit_stage<S: Stage>(mut self, state: T, stage: S) -> Self {
        self.set_exit_stage(state, stage);
        self
    }

    pub fn with_update_stage<S: Stage>(mut self, state: T, stage: S) -> Self {
        self.set_update_stage(state, stage);
        self
    }

    pub fn set_enter_stage<S: Stage>(&mut self, state: T, stage: S) -> &mut Self {
        let stages = self.state_stages(state);
        stages.enter = Box::new(stage);
        self
    }

    pub fn set_exit_stage<S: Stage>(&mut self, state: T, stage: S) -> &mut Self {
        let stages = self.state_stages(state);
        stages.exit = Box::new(stage);
        self
    }

    pub fn set_update_stage<S: Stage>(&mut self, state: T, stage: S) -> &mut Self {
        let stages = self.state_stages(state);
        stages.update = Box::new(stage);
        self
    }

    pub fn on_state_enter<S: System<In = (), Out = ()>>(
        &mut self,
        state: T,
        system: S,
    ) -> &mut Self {
        self.enter_stage(state, |system_stage: &mut SystemStage| {
            system_stage.add_system(system)
        })
    }

    pub fn on_state_exit<S: System<In = (), Out = ()>>(
        &mut self,
        state: T,
        system: S,
    ) -> &mut Self {
        self.exit_stage(state, |system_stage: &mut SystemStage| {
            system_stage.add_system(system)
        })
    }

    pub fn on_state_update<S: System<In = (), Out = ()>>(
        &mut self,
        state: T,
        system: S,
    ) -> &mut Self {
        self.update_stage(state, |system_stage: &mut SystemStage| {
            system_stage.add_system(system)
        })
    }

    pub fn enter_stage<S: Stage, F: FnOnce(&mut S) -> &mut S>(
        &mut self,
        state: T,
        func: F,
    ) -> &mut Self {
        let stages = self.state_stages(state);
        func(
            stages
                .enter
                .downcast_mut()
                .expect("'Enter' stage does not match the given type"),
        );
        self
    }

    pub fn exit_stage<S: Stage, F: FnOnce(&mut S) -> &mut S>(
        &mut self,
        state: T,
        func: F,
    ) -> &mut Self {
        let stages = self.state_stages(state);
        func(
            stages
                .exit
                .downcast_mut()
                .expect("'Exit' stage does not match the given type"),
        );
        self
    }

    pub fn update_stage<S: Stage, F: FnOnce(&mut S) -> &mut S>(
        &mut self,
        state: T,
        func: F,
    ) -> &mut Self {
        let stages = self.state_stages(state);
        func(
            stages
                .update
                .downcast_mut()
                .expect("'Update' stage does not match the given type"),
        );
        self
    }

    fn state_stages(&mut self, state: T) -> &mut StateStages {
        self.stages.entry(mem::discriminant(&state)).or_default()
    }
}

#[allow(clippy::mem_discriminant_non_enum)]
impl<T: Resource> Stage for StateStage<T> {
    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        for state_stages in self.stages.values_mut() {
            state_stages.enter.initialize(world, resources);
            state_stages.update.initialize(world, resources);
            state_stages.exit.initialize(world, resources);
        }
    }

    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        let current_stage = loop {
            let next = {
                let mut state = resources
                    .get_mut::<State<T>>()
                    .expect("Missing state resource");
                state.previous = state.apply_next().or_else(|| state.previous.take());
                mem::discriminant(&state.current)
            };
            if self.current_stage == Some(next) {
                break next;
            } else {
                if let Some(current_state_stages) =
                    self.current_stage.and_then(|it| self.stages.get_mut(&it))
                {
                    current_state_stages.exit.run(world, resources);
                }
                self.current_stage = Some(next);
                if let Some(next_state_stages) = self.stages.get_mut(&next) {
                    next_state_stages.enter.run(world, resources);
                }
            }
        };

        if let Some(current_state_stages) = self.stages.get_mut(&current_stage) {
            current_state_stages.update.run(world, resources);
        }
    }
}
#[derive(Debug, Error)]
pub enum StateError {
    #[error("Attempted to change the state to the current state.")]
    AlreadyInState,
    #[error("Attempted to queue a state change, but there was already a state queued.")]
    StateAlreadyQueued,
}

#[derive(Debug)]
pub struct State<T> {
    previous: Option<T>,
    current: T,
    next: Option<T>,
}

#[allow(clippy::mem_discriminant_non_enum)]
impl<T> State<T> {
    pub fn new(state: T) -> Self {
        Self {
            current: state,
            next: None,
            previous: None,
        }
    }

    pub fn current(&self) -> &T {
        &self.current
    }

    pub fn previous(&self) -> Option<&T> {
        self.previous.as_ref()
    }

    pub fn next(&self) -> Option<&T> {
        self.next.as_ref()
    }

    /// Queue a state change. This will fail if there is already a state in the queue, or if the given `state` matches the current state
    pub fn set_next(&mut self, state: T) -> Result<(), StateError> {
        if mem::discriminant(&self.current) == mem::discriminant(&state) {
            return Err(StateError::AlreadyInState);
        }

        if self.next.is_some() {
            return Err(StateError::StateAlreadyQueued);
        }

        self.next = Some(state);
        Ok(())
    }

    /// Same as [Self::queue], but there is already a next state, it will be overwritten instead of failing
    pub fn overwrite_next(&mut self, state: T) -> Result<(), StateError> {
        if mem::discriminant(&self.current) == mem::discriminant(&state) {
            return Err(StateError::AlreadyInState);
        }

        self.next = Some(state);
        Ok(())
    }

    fn apply_next(&mut self) -> Option<T> {
        if let Some(next) = self.next.take() {
            Some(std::mem::replace(&mut self.current, next))
        } else {
            None
        }
    }
}

impl<T> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.current
    }
}
