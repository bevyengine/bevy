use std::{cell::RefCell, hash::Hash};

use bevy_utils::HashMap;
use parking_lot::RwLock;
use std::ops::Deref;

use crate::{Component, IntoSystem, Resource, Resources, Stage, System, World};

pub struct StateStage<T> {
    stages: HashMap<T, Box<dyn Stage>>,
    transitions: HashMap<(T, T), Box<dyn System<Input = (), Output = ()>>>,
}

impl<T> Default for StateStage<T> {
    fn default() -> Self {
        Self {
            stages: Default::default(),
            transitions: Default::default(),
        }
    }
}

impl<T: Eq + Hash> StateStage<T> {
    pub fn state<S: Stage>(mut self, state: T, stage: S) -> Self {
        self.stages.insert(state, Box::new(stage));
        self
    }

    pub fn transition<S, Params, IntoS>(mut self, from: T, to: T, system: S) -> Self
    where
        S: System<Input = (), Output = ()>,
        IntoS: IntoSystem<Params, S>,
    {
        self.transitions
            .insert((from, to), Box::new(system.system()));
        self
    }
}

impl<T: Resource + Clone + Eq + Hash> Stage for StateStage<T> {
    fn run(&mut self, world: &mut World, resources: &mut Resources) {
        let state = resources.get_cloned::<T>().expect("Missing state resource");
        if let Some(stage) = self.stages.get_mut(&state) {
            stage.run(world, resources);
        }
    }
}

pub struct State<T: Clone + Hash + Eq + PartialEq> {
    current: T,
    change_queue: RwLock<Vec<T>>,
}

impl<T: Clone + Hash + Eq + PartialEq> State<T> {
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