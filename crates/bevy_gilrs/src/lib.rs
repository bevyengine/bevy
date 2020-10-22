mod converter;
mod gilrs_system;

use bevy_app::prelude::*;
use bevy_ecs::prelude::*;
use gilrs::GilrsBuilder;
use gilrs_system::gilrs_event_system;

#[derive(Default)]
pub struct GilrsPlugin;

impl Plugin for GilrsPlugin {
    fn build(&self, app: &mut AppBuilder) {
        match GilrsBuilder::new()
            .with_default_filters(false)
            .set_update_state(false)
            .build()
        {
            Ok(gilrs) => {
                app.add_thread_local_resource(gilrs)
                    .add_startup_system(gilrs_event_system.thread_local_system())
                    .add_system_to_stage(
                        stage::PRE_EVENT,
                        gilrs_event_system.thread_local_system(),
                    );
            }
            Err(err) => log::error!("Failed to start Gilrs. {}", err),
        }
    }
}
