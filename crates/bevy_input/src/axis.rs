use std::{
    collections::{hash_map::Entry, HashMap},
    hash::Hash,
};

#[derive(Debug)]
struct AxisData {
    current: f32,
    previous: f32,
}

#[derive(Debug)]
pub struct Axis<T> {
    data: HashMap<T, AxisData>,
}

impl<T> Default for Axis<T>
where
    T: Copy + Eq + Hash,
{
    fn default() -> Self {
        Axis {
            data: HashMap::new(),
        }
    }
}

impl<T> Axis<T>
where
    T: Copy + Eq + Hash,
{
    pub fn set(&mut self, axis: T, value: f32) {
        match self.data.entry(axis) {
            Entry::Occupied(mut occupied) => occupied.get_mut().current = value,
            Entry::Vacant(vacant) => {
                vacant.insert(AxisData {
                    current: value,
                    previous: value,
                });
            }
        }
    }

    pub fn current(&self, axis: T) -> Option<f32> {
        if let Some(data) = self.data.get(&axis) {
            Some(data.current)
        } else {
            None
        }
    }

    pub fn previous(&self, axis: T) -> Option<f32> {
        if let Some(data) = self.data.get(&axis) {
            Some(data.previous)
        } else {
            None
        }
    }

    pub fn delta(&self, axis: T) -> Option<f32> {
        if let Some(data) = self.data.get(&axis) {
            Some(data.current - data.previous)
        } else {
            None
        }
    }

    pub fn remove(&mut self, axis: T) {
        self.data.remove(&axis);
    }

    pub fn update(&mut self) {
        for (_, data) in self.data.iter_mut() {
            data.previous = data.current;
        }
    }
}
