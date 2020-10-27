//! Input Plugin Module.
//! Implements a bevy plugin that adds the resources and systems for mouse, keyboard_devices, touch, and gamepad_device input

use crate::{
    gamepad::GamepadInputPlugin, keyboard::KeyboardInputPlugin, mouse::MouseInputPlugin,
    touch::TouchInputPlugin,
};
use bevy_app::prelude::*;

/// Adds keyboard_devices, mouse, gamepad_device, and touch input to an App
#[derive(Default)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(KeyboardInputPlugin)
            .add_plugin(MouseInputPlugin)
            .add_plugin(GamepadInputPlugin)
            .add_plugin(TouchInputPlugin);
    }
}
