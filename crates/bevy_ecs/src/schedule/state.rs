use crate::{Resource, Resources, Stage, World};
use bevy_utils::{tracing::trace, HashMap};
use parking_lot::RwLock;
use std::{hash::Hash, ops::Deref};

#[derive(Default)]
pub(crate) struct StateStages {
    update: Option<Box<dyn Stage>>,
    enter: Option<Box<dyn Stage>>,
    exit: Option<Box<dyn Stage>>,
}

pub struct StateStage<T> {
    // TODO: consider making this an array
    stages: HashMap<T, StateStages>,
}

impl<T> Default for StateStage<T> {
    fn default() -> Self {
        Self {
            stages: Default::default(),
        }
    }
}

impl<T: Eq + Hash> StateStage<T> {
    pub fn set_enter<S: Stage>(&mut self, state: T, stage: S) {
        let stages = self
            .stages
            .entry(state)
            .or_insert_with(|| StateStages::default());
        stages.enter = Some(Box::new(stage));
    }

    pub fn set_exit<S: Stage>(&mut self, state: T, stage: S) {
        let stages = self
            .stages
            .entry(state)
            .or_insert_with(|| StateStages::default());
        stages.exit = Some(Box::new(stage));
    }

    pub fn set_update<S: Stage>(&mut self, state: T, stage: S) {
        let stages = self
            .stages
            .entry(state)
            .or_insert_with(|| StateStages::default());
        stages.update = Some(Box::new(stage));
    }

    pub fn run_enter(&mut self, state: &T, world: &mut World, resources: &mut Resources) {
        if let Some(enter) = self
            .stages
            .get_mut(&state)
            .and_then(|stage| stage.enter.as_mut())
        {
            enter.run(world, resources);
        }
    }

    pub fn run_update(&mut self, state: &T, world: &mut World, resources: &mut Resources) {
        if let Some(update) = self
            .stages
            .get_mut(&state)
            .and_then(|stage| stage.update.as_mut())
        {
            update.run(world, resources);
        }
    }

    pub fn run_exit(&mut self, state: &T, world: &mut World, resources: &mut Resources) {
        if let Some(exit) = self
            .stages
            .get_mut(&state)
            .and_then(|stage| stage.exit.as_mut())
        {
            exit.run(world, resources);
        }
    }
}

impl<T: Resource + Clone + Eq + Hash> Stage for StateStage<T> {
    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        let (previous_state, mut current_state, change_queue) = {
            let mut state = resources
                .get_mut::<State<T>>()
                .expect("Missing state resource");
            // we use slightly roundabout scoping here to ensure the State resource only
            // gets set to "mutated" when the state actually changes
            let (next_state, change_queue) = {
                let mut change_queue = state.change_queue.write();
                let mut next_state = None;
                if !change_queue.is_empty() {
                    next_state = Some(change_queue[change_queue.len() - 1].clone());
                }
                (next_state, std::mem::take(&mut *change_queue))
            };
            let previous_state = state.current.clone();
            if let Some(next_state) = next_state {
                state.current = next_state;
            }
            (previous_state, state.get(), change_queue)
        };
        for next_state in change_queue {
            if next_state != previous_state {
                println!("exit");
                self.run_exit(&current_state, world, resources);
            } else {
                println!("not exiting");
            }

            println!("enter");
            self.run_enter(&next_state, world, resources);
            current_state = next_state
        }

        println!("update");
        self.run_update(&current_state, world, resources);
    }
}

pub struct State<T: Clone + Hash + Eq + PartialEq> {
    current: T,
    change_queue: RwLock<Vec<T>>,
}

impl<T: Clone + Hash + Eq + PartialEq> State<T> {
    pub fn new(value: T) -> Self {
        Self {
            current: value.clone(),
            // add value to queue so that we "enter" the state
            change_queue: RwLock::new(vec![value]),
        }
    }

    pub fn get(&self) -> T {
        self.current.clone()
    }

    pub fn queue_change(&self, value: T) {
        if self.current == value {
            return;
        }
        let mut change_queue = self.change_queue.write();
        if !change_queue.is_empty() && change_queue[change_queue.len() - 1] == value {
            return;
        }
        change_queue.push(value);
    }
}

impl<T: Clone + Hash + Eq + PartialEq> Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.current
    }
}
