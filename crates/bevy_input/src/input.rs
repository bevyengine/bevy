use bevy_utils::{HashMap, HashSet};
use std::hash::Hash;

/// A "press-able" input of type `T`
#[derive(Debug)]
pub struct Input<T> {
    pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    just_released: HashSet<T>,
    values: HashMap<T, f32>,
}

impl<T> Default for Input<T> {
    fn default() -> Self {
        Self {
            pressed: Default::default(),
            just_pressed: Default::default(),
            just_released: Default::default(),
            values: Default::default(),
        }
    }
}

impl<T> Input<T>
where
    T: Copy + Eq + Hash,
{
    pub fn press(&mut self, input: T) {
        if !self.pressed(input) {
            self.just_pressed.insert(input);
        }

        self.pressed.insert(input);
    }

    pub fn pressed(&self, input: T) -> bool {
        self.pressed.contains(&input)
    }

    pub fn release(&mut self, input: T) {
        self.pressed.remove(&input);
        self.just_released.insert(input);
    }

    pub fn just_pressed(&self, input: T) -> bool {
        self.just_pressed.contains(&input)
    }

    pub fn just_released(&self, input: T) -> bool {
        self.just_released.contains(&input)
    }

    pub fn reset(&mut self, input: T) {
        self.pressed.remove(&input);
        self.just_pressed.remove(&input);
        self.just_released.remove(&input);
    }

    pub fn set_value(&mut self, input: T, value: f32) {
        if value > 0f32 {
            self.values.insert(input, value);
        } else {
            self.values.remove(&input);
        }
    }

    pub fn value(&self, input: T) -> Option<f32> {
        self.values.get(&input).cloned()
    }

    pub fn update(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }

    pub fn get_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.pressed.iter()
    }

    pub fn get_just_pressed(&self) -> impl ExactSizeIterator<Item = &T> {
        self.just_pressed.iter()
    }

    pub fn get_just_released(&self) -> impl ExactSizeIterator<Item = &T> {
        self.just_released.iter()
    }
}
