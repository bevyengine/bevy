pub mod inputmap;
pub mod keyboard;

use crate::inputmap::InputMap;
use bevy_app::prelude::*;
use bevy_ecs::IntoQuerySystem;
use keyboard::KeyboardMap;

#[derive(Default)]
pub struct InputMapPlugin;

impl Plugin for InputMapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<InputMap>()

            // keyboard
            .init_resource::<KeyboardMap>()
            .add_system_to_stage(stage::EVENT_UPDATE, KeyboardMap::action_reset_system.system())
            .add_system_to_stage(stage::UPDATE, KeyboardMap::action_update_system.system())
            ;
    }
}
