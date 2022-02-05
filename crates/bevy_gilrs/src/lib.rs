mod converter;
mod gilrs_system;
mod rumble;

use bevy_app::{App, CoreStage, Plugin, StartupStage};
use bevy_ecs::schedule::ParallelSystemDescriptorCoercion;
use bevy_input::InputSystem;
use bevy_utils::tracing::error;
pub use gilrs::ff;
use gilrs::GilrsBuilder;
use gilrs_system::{gilrs_event_startup_system, gilrs_event_system};
pub use rumble::{RumbleIntensity, RumbleRequest};

#[derive(Default)]
pub struct GilrsPlugin;

impl Plugin for GilrsPlugin {
    fn build(&self, app: &mut App) {
        match GilrsBuilder::new()
            .with_default_filters(false)
            .set_update_state(false)
            .build()
        {
            Ok(gilrs) => {
                app.insert_non_send_resource(gilrs)
                    .add_event::<RumbleRequest>()
                    .init_non_send_resource::<rumble::RumblesManager>()
                    .add_startup_system_to_stage(
                        StartupStage::PreStartup,
                        gilrs_event_startup_system,
                    )
                    .add_system_to_stage(CoreStage::PostUpdate, rumble::gilrs_rumble_system)
                    .add_system_to_stage(
                        CoreStage::PreUpdate,
                        gilrs_event_system.before(InputSystem),
                    );
            }
            Err(err) => error!("Failed to start Gilrs. {}", err),
        }
    }
}
