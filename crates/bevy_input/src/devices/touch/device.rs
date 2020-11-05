use super::*;
use bevy_math::Vec2;
use bevy_utils::HashMap;

#[derive(Debug, Clone, Copy)]
pub struct Touch {
    pub id: u64,
    pub start_position: Vec2,
    pub start_force: Option<ForceTouch>,
    pub previous_position: Vec2,
    pub previous_force: Option<ForceTouch>,
    pub position: Vec2,
    pub force: Option<ForceTouch>,
}

impl Touch {
    pub fn delta(&self) -> Vec2 {
        self.position - self.previous_position
    }

    pub fn distance(&self) -> Vec2 {
        self.position - self.start_position
    }
}

impl From<&TouchEvent> for Touch {
    fn from(input: &TouchEvent) -> Touch {
        Touch {
            id: input.id,
            start_position: input.position,
            start_force: input.force,
            previous_position: input.position,
            previous_force: input.force,
            position: input.position,
            force: input.force,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Touches {
    pub(crate) pressed: HashMap<u64, Touch>,
    pub(crate) just_pressed: HashMap<u64, Touch>,
    pub(crate) just_released: HashMap<u64, Touch>,
    pub(crate) just_cancelled: HashMap<u64, Touch>,
}

impl Touches {
    pub fn iter(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.pressed.values()
    }

    pub fn get_pressed(&self, id: u64) -> Option<&Touch> {
        self.pressed.get(&id)
    }

    pub fn just_pressed(&self, id: u64) -> bool {
        self.just_pressed.contains_key(&id)
    }

    pub fn iter_just_pressed(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_pressed
            .iter()
            .map(move |(id, _)| self.pressed.get(id).unwrap())
    }

    pub fn get_released(&self, id: u64) -> Option<&Touch> {
        self.just_released.get(&id)
    }

    pub fn just_released(&self, id: u64) -> bool {
        self.just_released.contains_key(&id)
    }

    pub fn iter_just_released(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_released
            .iter()
            .map(move |(id, _)| self.pressed.get(id).unwrap())
    }

    pub fn just_cancelled(&self, id: u64) -> bool {
        self.just_cancelled.contains_key(&id)
    }

    pub fn iter_just_cancelled(&self) -> impl Iterator<Item = &Touch> + '_ {
        self.just_cancelled
            .iter()
            .map(move |(id, _)| self.pressed.get(id).unwrap())
    }
}
