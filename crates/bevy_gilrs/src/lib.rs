mod converter;
mod gilrs_system;

use bevy_app::{prelude::*, startup_stage::PRE_STARTUP};
use bevy_ecs::IntoSystem;
use bevy_utils::tracing::error;
use gilrs::GilrsBuilder;
use gilrs_system::{gilrs_event_startup_system, gilrs_event_system};

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
                    .add_startup_system_to_stage(PRE_STARTUP, gilrs_event_startup_system.system())
                    .add_system_to_stage(stage::PRE_EVENT, gilrs_event_system.system());
            }
            Err(err) => error!("Failed to start Gilrs. {}", err),
        }
    }
}
