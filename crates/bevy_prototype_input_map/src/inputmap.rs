use std::collections::{HashSet, HashMap};
use bevy_input::{mouse::MouseButton, prelude::KeyCode};

pub struct InputMap
{
    // actions
    action: HashSet<String>,

    // inputs
    keyboard_pressed_map: HashMap<KeyCode, String>,
    keyboard_just_pressed_map: HashMap<KeyCode, String>,
    keyboard_just_released_map: HashMap<KeyCode, String>,
    
    mouse_pressed_map: HashMap<MouseButton, String>,
    mouse_just_pressed_map: HashMap<MouseButton, String>,
    mouse_just_released_map: HashMap<MouseButton, String>,
}

impl InputMap
{
    // keyboard
    pub fn BindKeyboardPressed(&mut self, code: KeyCode, action: String)
    {
        self.keyboard_pressed_map.insert(
            code,
            action
            );
    }
    pub fn BindKeyboardJustPressed(&mut self, code: KeyCode, action: String)
    {
        self.keyboard_just_pressed_map.insert(
            code,
            action
            );
    }
    pub fn BindKeyboardJustReleased(&mut self, code: KeyCode, action: String)
    {
        self.keyboard_just_released_map.insert(
            code,
            action
            );
    }
    pub fn UnBindKeyboardPressed(&mut self, code: KeyCode, action: String)
    {
        self.keyboard_pressed_map.remove(
            code,
            action
            );
    }
    pub fn UnBindKeyboardJustPressed(&mut self, code: KeyCode, action: String)
    {
        self.keyboard_just_pressed_map.remove(
            code,
            action
            );
    }
    pub fn UnBindKeyboardJustReleased(&mut self, code: KeyCode, action: String)
    {
        self.keyboard_just_released_map.remove(
            code,
            action
            );
    }
}