use std::{collections::HashSet, hash::Hash};

pub struct Input<T> {
    pressed: HashSet<T>,
    just_pressed: HashSet<T>,
    just_released: HashSet<T>,
}

impl<T> Default for Input<T> {
    fn default() -> Self {
        Self {
            pressed: Default::default(),
            just_pressed: Default::default(),
            just_released: Default::default(),
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

    pub fn update(&mut self) {
        self.just_pressed.clear();
        self.just_released.clear();
    }
}
