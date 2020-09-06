pub mod inputmap;
pub mod keyboard;

use crate::inputmap::InputMap;
use bevy_app::prelude::*;
use bevy_input::{prelude::KeyCode, Input};
use bevy_ecs::{ResMut, IntoQuerySystem};
use keyboard::KeyboardMap;

#[derive(Default)]
pub struct InputMapPlugin;

impl Plugin for InputMapPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.init_resource::<InputMap>()
            .init_resource::<KeyboardMap>()
            .add_system_to_stage(
                bevy_app::stage::EVENT_UPDATE,
                KeyboardMap::action_system.system()
            )
            ;
    }
}
