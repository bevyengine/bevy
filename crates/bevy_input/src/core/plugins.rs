//! Core plugins module
//!
//! This module provides convenience [plugins](https://bevyengine.org/learn/book/getting-started/plugins/)
//! for setting up one of the supported input devices
//!
//! High Level supported device list:
//!  - Keyboard
//!  - Mouse
//!  - Touch (touchscreen etc.)
//!  - Gamepad
//!
//! for more information on how to implement these devices see:
//!  - [Bevy Input Examples](https://github.com/bevyengine/bevy/tree/master/examples/input)
//!

use super::*;
use crate::devices::*;
use bevy_app::*;
use bevy_ecs::IntoQuerySystem;

/// Adds input device support to an App
#[derive(Debug, Default)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_plugin(KeyboardPlugin)
            .add_plugin(MousePlugin)
            .add_plugin(GamepadPlugin)
            .add_plugin(TouchPlugin);
    }
}

impl PluginGroup for InputPlugin {
    fn build(&mut self, group: &mut bevy_app::PluginGroupBuilder) {
        group
            .add(KeyboardPlugin)
            .add(MousePlugin)
            .add(GamepadPlugin)
            .add(TouchPlugin);
    }
}

/// Adds gamepad input to an App
#[derive(Default)]
pub struct GamepadPlugin;

impl Plugin for GamepadPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<GamepadEvent>()
            .add_event::<GamepadEventRaw>()
            .init_resource::<GamepadSettings>()
            .init_resource::<BinaryInput<GamepadButton>>()
            .init_resource::<Axis<GamepadAxis>>()
            .init_resource::<Axis<GamepadButton>>()
            .add_system_to_stage(bevy_app::stage::EVENT, gamepad_event_system.system());
    }
}

/// Adds keyboard input to an App
#[derive(Default)]
pub struct KeyboardPlugin;

impl Plugin for KeyboardPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<KeyboardEvent>()
            .init_resource::<BinaryInput<KeyCode>>()
            .add_system_to_stage(bevy_app::stage::EVENT, keyboard_input_system.system());
    }
}

/// Adds mouse input to an App
#[derive(Default)]
pub struct MousePlugin;

impl Plugin for MousePlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<MouseButtonEvent>()
            .add_event::<MouseMotionEvent>()
            .add_event::<MouseWheelEvent>()
            .init_resource::<BinaryInput<MouseButtonCode>>()
            .add_system_to_stage(bevy_app::stage::EVENT, mouse_button_input_system.system());
    }
}

/// Adds touch input to an App
#[derive(Default)]
pub struct TouchPlugin;

impl Plugin for TouchPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<TouchEvent>()
            .init_resource::<Touches>()
            .add_system_to_stage(bevy_app::stage::EVENT, touch_screen_input_system.system());
    }
}
