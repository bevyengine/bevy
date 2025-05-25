//! Utilities for hotpatching code.

use std::sync::Arc;

use bevy_ecs::{event::EventWriter, HotPatched};
#[cfg(not(target_family = "wasm"))]
use dioxus_devtools::connect_subsecond;
use dioxus_devtools::subsecond;

pub use dioxus_devtools::subsecond::{call, HotFunction};

use crate::{Last, Plugin};

/// Plugin connecting to Dioxus CLI to enable hot patching.
#[derive(Default)]
pub struct HotPatchPlugin;

impl Plugin for HotPatchPlugin {
    fn build(&self, app: &mut crate::App) {
        let (sender, receiver) = crossbeam_channel::bounded::<HotPatched>(1);

        // Connects to the dioxus CLI that will handle rebuilds
        // This will open a connection to the dioxus CLI to receive updated jump tables
        // Sends a `HotPatched` message through the channel when the jump table is updated
        #[cfg(not(target_family = "wasm"))]
        connect_subsecond();
        subsecond::register_handler(Arc::new(move || {
            _ = sender.send(HotPatched).unwrap();
        }));

        // Adds a system that will read the channel for new `HotPatched`, and forward them as event to the ECS
        app.add_event::<HotPatched>().add_systems(
            Last,
            move |mut events: EventWriter<HotPatched>| {
                if receiver.try_recv().is_ok() {
                    events.write_default();
                }
            },
        );
    }
}
