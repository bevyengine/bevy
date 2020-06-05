use crate::keyboard::{KeyCode, KeyboardInput, ElementState};
use bevy_app::{EventReader, Events};
use legion::prelude::{Res, ResMut};
use std::collections::HashSet;

#[derive(Default)]
pub struct Input {
    pressed_keys: HashSet<KeyCode>,
    just_pressed_keys: HashSet<KeyCode>,
    just_released_keys: HashSet<KeyCode>,
}

impl Input {
    pub fn press_key(&mut self, key_code: KeyCode) {
        if !self.key_pressed(key_code) {
            self.just_pressed_keys.insert(key_code);
        }

        self.pressed_keys.insert(key_code);
    }

    pub fn release_key(&mut self, key_code: KeyCode) {
        self.pressed_keys.remove(&key_code);
        self.just_released_keys.insert(key_code);
    }

    pub fn key_pressed(&self, key_code: KeyCode) -> bool {
        self.pressed_keys.contains(&key_code)
    }

    pub fn key_just_pressed(&self, key_code: KeyCode) -> bool {
        self.just_pressed_keys.contains(&key_code)
    }

    pub fn key_just_released(&self, key_code: KeyCode) -> bool {
        self.just_released_keys.contains(&key_code)
    }

    pub fn update(&mut self) {
        self.just_pressed_keys.clear();
        self.just_released_keys.clear();
    }
}

#[derive(Default)]
pub struct InputState {
    keyboard_input_event_reader: EventReader<KeyboardInput>,
}

pub fn input_system(
    mut state: ResMut<InputState>,
    mut input: ResMut<Input>,
    keyboard_input_events: Res<Events<KeyboardInput>>,
) {
    input.update();
    for event in state
        .keyboard_input_event_reader
        .iter(&keyboard_input_events)
    {
        if let KeyboardInput {
            key_code: Some(key_code),
            state,
            ..
        } = event
        {
            match state {
                ElementState::Pressed => input.press_key(*key_code),
                ElementState::Released => input.release_key(*key_code),
            }
        }
    }
}
