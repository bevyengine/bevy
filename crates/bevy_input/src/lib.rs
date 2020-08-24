mod axis;
mod input;
pub mod keyboard;
pub mod mouse;
pub mod system;

pub use axis::*;
pub use input::*;
pub use mouse::Mouse;

pub mod prelude {
    pub use crate::{keyboard::KeyCode, mouse::MouseButton, Axis, Input, Mouse};
}

use bevy_app::prelude::*;
use bevy_window::{AxisId, Cursor};
use keyboard::{keyboard_input_system, KeyCode, KeyboardInput};

use mouse::{
    axis_system, cursor_system, mouse_axis_system, mouse_button_input_system, MouseButton,
    MouseButtonInput, MouseMotion, MouseWheel,
};

use bevy_ecs::IntoQuerySystem;

/// Adds keyboard and mouse input to an App
#[derive(Default)]
pub struct InputPlugin;

impl Plugin for InputPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_event::<KeyboardInput>()
            .add_event::<MouseButtonInput>()
            .add_event::<MouseMotion>()
            .add_event::<MouseWheel>()
            .init_resource::<Input<KeyCode>>()
            .add_system_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                keyboard_input_system.system(),
            )
            .init_resource::<Input<MouseButton>>()
            .add_system_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                mouse_button_input_system.system(),
            )
            .init_resource::<Axis<Cursor>>()
            .add_system_to_stage(bevy_app::stage::EVENT_UPDATE, cursor_system.system())
            .init_resource::<Axis<Mouse>>()
            .add_system_to_stage(bevy_app::stage::EVENT_UPDATE, mouse_axis_system.system())
            .init_resource::<Axis<AxisId>>()
            .add_system_to_stage(bevy_app::stage::EVENT_UPDATE, axis_system.system());
    }
}
