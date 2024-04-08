#![cfg_attr(docsrs, feature(doc_auto_cfg))]
#![forbid(unsafe_code)]
#![doc(
    html_logo_url = "https://bevyengine.org/assets/icon.png",
    html_favicon_url = "https://bevyengine.org/assets/icon.png"
)]

//! Systems and type definitions for gamepad handling in Bevy.
//!
//! This crate is built on top of [GilRs](gilrs), a library
//! that handles abstracting over platform-specific gamepad APIs.

mod converter;
mod gilrs_system;
mod rumble;

use bevy_app::{App, Plugin, PostUpdate, PreStartup, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_input::InputSystem;
use bevy_utils::{synccell::SyncCell, tracing::error};
use gilrs::GilrsBuilder;
use gilrs_system::{gilrs_event_startup_system, gilrs_event_system};
use rumble::{play_gilrs_rumble, RunningRumbleEffects};

#[cfg_attr(not(target_arch = "wasm32"), derive(Resource))]
pub(crate) struct Gilrs(pub SyncCell<gilrs::Gilrs>);

/// Plugin that provides gamepad handling to an [`App`].
#[derive(Default)]
pub struct GilrsPlugin;

/// Updates the running gamepad rumble effects.
#[derive(Debug, PartialEq, Eq, Clone, Hash, SystemSet)]
pub struct RumbleSystem;

impl Plugin for GilrsPlugin {
    fn build(&self, app: &mut App) {
        match GilrsBuilder::new()
            .with_default_filters(false)
            .set_update_state(false)
            .build()
        {
            Ok(gilrs) => {
                #[cfg(target_arch = "wasm32")]
                app.insert_non_send_resource(Gilrs(SyncCell::new(gilrs)));
                #[cfg(not(target_arch = "wasm32"))]
                app.insert_resource(Gilrs(SyncCell::new(gilrs)));

                app.init_resource::<RunningRumbleEffects>()
                    .add_systems(PreStartup, gilrs_event_startup_system)
                    .add_systems(PreUpdate, gilrs_event_system.before(InputSystem))
                    .add_systems(PostUpdate, play_gilrs_rumble.in_set(RumbleSystem));
            }
            Err(err) => error!("Failed to start Gilrs. {}", err),
        }
    }
}
