use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
};

#[derive(Debug)]
pub struct Axis<T> {
    current: HashMap<T, f32>,
    previous: HashMap<T, f32>,
}

impl<T> Default for Axis<T>
where
    T: Copy + Eq + Hash,
{
    fn default() -> Self {
        Axis {
            current: HashMap::new(),
            previous: HashMap::new(),
        }
    }
}

impl<T> Axis<T>
where
    T: Copy + Eq + Hash,
{
    pub fn set(&mut self, axis: T, value: f32) {
        if let Entry::Vacant(vacant) = self.previous.entry(axis) {
            if let Some(current) = self.current.get(&axis) {
                vacant.insert(*current);
            }
        }
        self.current.insert(axis, value);
    }

    pub fn current(&self, axis: T) -> Option<f32> {
        self.current.get(&axis).copied()
    }

    pub fn previous(&self, axis: T) -> Option<f32> {
        self.previous.get(&axis).copied()
    }

    pub fn delta(&self, axis: T) -> Option<f32> {
        if let (Some(current), Some(previous)) = (self.current.get(&axis), self.previous.get(&axis))
        {
            Some(current - previous)
        } else {
            None
        }
    }

    pub fn remove(&mut self, axis: T) {
        self.current.remove(&axis);
        self.previous.remove(&axis);
    }

    pub fn update(&mut self) {
        self.previous.clear();
    }
}
