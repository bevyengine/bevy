use std::hash::Hash;
use bevy_utils::{AhashMap, AhashExt};

#[derive(Debug)]
pub struct Axis<T> {
    axis_data: AhashMap<T, f32>,
}

impl<T> Default for Axis<T>
where
    T: Copy + Eq + Hash,
{
    fn default() -> Self {
        Axis {
            axis_data: AhashMap::new(),
        }
    }
}

impl<T> Axis<T>
where
    T: Copy + Eq + Hash,
{
    pub fn set(&mut self, axis: T, value: f32) -> Option<f32> {
        self.axis_data.insert(axis, value)
    }

    pub fn get(&self, axis: T) -> Option<f32> {
        self.axis_data.get(&axis).copied()
    }

    pub fn remove(&mut self, axis: T) -> Option<f32> {
        self.axis_data.remove(&axis)
    }
}
