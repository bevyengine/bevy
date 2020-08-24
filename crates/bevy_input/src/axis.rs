use std::{collections::HashMap, hash::Hash};

pub struct Axis<T> {
    axes: HashMap<T, f32>,
}

impl<T> Default for Axis<T> {
    fn default() -> Self {
        Self {
            axes: Default::default(),
        }
    }
}

impl<T> Axis<T>
where
    T: Copy + Eq + Hash,
{
    pub fn register(&mut self, axis_id: T) {
        self.axes.insert(axis_id, 0.0);
    }

    pub fn set(&mut self, axis_id: T, value: f32) {
        if let Some(axis) = self.axes.get_mut(&axis_id) {
            *axis = value;
        }
    }

    pub fn add(&mut self, axis_id: T, value: f32) {
        if let Some(axis) = self.axes.get_mut(&axis_id) {
            *axis += value;
        }
    }

    pub fn get(&self, axis_id: T) -> Option<f32> {
        match self.axes.get(&axis_id) {
            Some(axis) => Some(*axis),
            None => None,
        }
    }
}
