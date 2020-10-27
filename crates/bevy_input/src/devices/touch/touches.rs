use bevy_app::{EventReader, Events};
use bevy_ecs::{Local, Res, ResMut};
use bevy_math::Vec2;
use bevy_utils::{HashMap, HashSet};
use crate::devices::touch::Touch;

#[derive(Default)]
pub struct Touches {
    active_touches: HashMap<u64, Touch>,
    just_pressed: HashSet<u64>,
    just_released: HashSet<u64>,
    just_cancelled: HashSet<u64>,
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
