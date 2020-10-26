//! Input Plugin Module.
//! Implements a bevy plugin that adds the resources and systems for mouse, keyboard, touch, and gamepad input

use super::prelude::*;
use bevy_app::prelude::*;

/// Adds keyboard, mouse, gamepad, and touch input to an App
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
