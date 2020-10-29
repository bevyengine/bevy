use bevy_math::Vec2;
use bevy_utils::{HashMap, HashSet};

#[derive(Debug, Clone)]
pub struct Touch {
    pub id: u64,
    pub start_position: Vec2,
    pub previous_position: Vec2,
    pub position: Vec2,
}

impl Touch {
    pub fn delta(&self) -> Vec2 {
        self.position - self.previous_position
    }

    pub fn distance(&self) -> Vec2 {
        self.position - self.start_position
    }
}

#[derive(Default)]
pub struct Touches {
    pub(crate) active_touches: HashMap<u64, Touch>,
    pub(crate) just_pressed: HashSet<u64>,
    pub(crate) just_released: HashSet<u64>,
    pub(crate) just_cancelled: HashSet<u64>,
}

impl Touches {
    pub fn iter(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.active_touches.values()
    }

    pub fn just_pressed(&self, id: u64) -> bool {
        self.just_pressed.contains(&id)
    }

    pub fn iter_just_pressed(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_pressed
            .iter()
            .map(move |id| self.active_touches.get(id).unwrap())
    }

    pub fn just_released(&self, id: u64) -> bool {
        self.just_released.contains(&id)
    }

    pub fn iter_just_released(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_released
            .iter()
            .map(move |id| self.active_touches.get(id).unwrap())
    }

    pub fn just_cancelled(&self, id: u64) -> bool {
        self.just_cancelled.contains(&id)
    }

    pub fn iter_just_cancelled(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_cancelled
            .iter()
            .map(move |id| self.active_touches.get(id).unwrap())
    }
}
