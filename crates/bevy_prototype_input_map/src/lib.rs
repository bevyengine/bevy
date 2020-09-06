pub mod inputmap;
pub mod keyboard;
pub mod mouse;
pub mod axis;

use crate::inputmap::InputMap;
use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;
use keyboard::KeyboardMap;
use mouse::MouseMap;

#[derive(Default)]
pub struct InputMapPlugin;

impl Plugin for InputMapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app
            // input map
            .init_resource::<InputMap>()
            .add_system_to_stage(stage::EVENT_UPDATE, InputMap::action_reset_system.system())

            // keyboard
            .init_resource::<KeyboardMap>()
            .add_system_to_stage(stage::UPDATE, KeyboardMap::action_update_system.system())

            // mouse
            .init_resource::<MouseMap>()
            .add_system_to_stage(stage::UPDATE, MouseMap::action_update_system.system())
            ;
    }
}
