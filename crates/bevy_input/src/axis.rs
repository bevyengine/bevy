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
    pub fn set(&mut self, axis_id: T, value: f32) {
        match self.axes.get_mut(&axis_id) {
            Some(axis) => {
                *axis = value;
            }
            None => (), // Panic?? Or Result?
        }
    }

    pub fn add(&mut self, axis_id: T, value: f32) {
        match self.axes.get_mut(&axis_id) {
            Some(axis) => {
                *axis += value;
            }
            None => (), // Panic?? Or Result?
        }
    }

    pub fn get(&self, axis_id: T) -> Option<f32> {
        match self.axes.get(&axis_id) {
            Some(axis) => Some(*axis),
            None => None,
        }
    }
}
