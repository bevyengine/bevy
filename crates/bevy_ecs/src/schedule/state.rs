use crate::{IntoStage, Resource, Resources, Stage, World};
use bevy_utils::HashMap;
use std::{mem::Discriminant, ops::Deref};
use thiserror::Error;

#[derive(Default)]
pub(crate) struct StateStages {
    update: Option<Box<dyn Stage>>,
    enter: Option<Box<dyn Stage>>,
    exit: Option<Box<dyn Stage>>,
}

pub struct StateStage<T> {
    stages: HashMap<Discriminant<T>, StateStages>,
}

impl<T> Default for StateStage<T> {
    fn default() -> Self {
        Self {
            stages: Default::default(),
        }
    }
}

#[allow(clippy::mem_discriminant_non_enum)]
impl<T> StateStage<T> {
    pub fn with_on_state_enter<Params, S: IntoStage<Params>>(mut self, state: T, stage: S) -> Self {
        self.on_state_enter(state, stage);
        self
    }

    pub fn with_on_state_exit<Params, S: IntoStage<Params>>(mut self, state: T, stage: S) -> Self {
        self.on_state_exit(state, stage);
        self
    }

    pub fn with_on_state_update<Params, S: IntoStage<Params>>(
        mut self,
        state: T,
        stage: S,
    ) -> Self {
        self.on_state_update(state, stage);
        self
    }

    pub fn on_state_enter<Params, S: IntoStage<Params>>(
        &mut self,
        state: T,
        stage: S,
    ) -> &mut Self {
        let stages = self
            .stages
            .entry(std::mem::discriminant(&state))
            .or_default();
        stages.enter = Some(Box::new(stage.into_stage()));
        self
    }

    pub fn on_state_exit<Params, S: IntoStage<Params>>(&mut self, state: T, stage: S) -> &mut Self {
        let stages = self
            .stages
            .entry(std::mem::discriminant(&state))
            .or_default();
        stages.exit = Some(Box::new(stage.into_stage()));
        self
    }

    pub fn on_state_update<Params, S: IntoStage<Params>>(
        &mut self,
        state: T,
        stage: S,
    ) -> &mut Self {
        let stages = self
            .stages
            .entry(std::mem::discriminant(&state))
            .or_default();
        stages.update = Some(Box::new(stage.into_stage()));
        self
    }
}

#[allow(clippy::mem_discriminant_non_enum)]
impl<T: Resource + Clone> Stage for StateStage<T> {
    fn initialize(&mut self, world: &mut World, resources: &mut Resources) {
        for state_stages in self.stages.values_mut() {
            if let Some(ref mut enter) = state_stages.enter {
                enter.initialize(world, resources);
            }

            if let Some(ref mut update) = state_stages.update {
                update.initialize(world, resources);
            }

            if let Some(ref mut exit) = state_stages.exit {
                exit.initialize(world, resources);
            }
        }
    }

    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        loop {
            let (next_stage, current_stage) = {
                let mut state = resources
                    .get_mut::<State<T>>()
                    .expect("Missing state resource");
                let result = (
                    state.next.as_ref().map(|next| std::mem::discriminant(next)),
                    std::mem::discriminant(&state.current),
                );

                state.apply_next();

                result
            };

            // if next_stage is Some, we just applied a new state
            if let Some(next_stage) = next_stage {
                if next_stage != current_stage {
                    if let Some(exit_current) = self
                        .stages
                        .get_mut(&current_stage)
                        .and_then(|stage| stage.exit.as_mut())
                    {
                        exit_current.run(world, resources);
                    }
                }

                if let Some(enter_next) = self
                    .stages
                    .get_mut(&next_stage)
                    .and_then(|stage| stage.enter.as_mut())
                {
                    enter_next.run(world, resources);
                }
            } else if let Some(update_current) = self
                .stages
                .get_mut(&current_stage)
                .and_then(|stage| stage.update.as_mut())
            {
                update_current.run(world, resources);
                break;
            }
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
pub struct State<T: Clone> {
    previous: Option<T>,
    current: T,
    next: Option<T>,
}

#[allow(clippy::mem_discriminant_non_enum)]
impl<T: Clone> State<T> {
    pub fn new(state: T) -> Self {
        Self {
            current: state.clone(),
            previous: None,
            // add value to queue so that we "enter" the state
            next: Some(state),
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
        if std::mem::discriminant(&self.current) == std::mem::discriminant(&state) {
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
        if std::mem::discriminant(&self.current) == std::mem::discriminant(&state) {
            return Err(StateError::AlreadyInState);
        }

        self.next = Some(state);
        Ok(())
    }

    fn apply_next(&mut self) {
        if let Some(next) = self.next.take() {
            let previous = std::mem::replace(&mut self.current, next);
            if std::mem::discriminant(&previous) != std::mem::discriminant(&self.current) {
                self.previous = Some(previous)
            }
        }
    }
}

impl<T: Clone> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.current
    }
}
