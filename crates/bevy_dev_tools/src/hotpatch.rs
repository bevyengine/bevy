//! Utilities for hotpatching code.

use bevy_ecs::{event::EventWriter, HotPatched};
use dioxus_devtools::{connect, subsecond::apply_patch, DevserverMsg};

pub use dioxus_devtools::subsecond::{call, HotFunction};

use crate::{Last, Plugin};

/// Plugin connecting to Dioxus CLI to enable hot patching.
#[derive(Default)]
pub struct HotPatchPlugin;

impl Plugin for HotPatchPlugin {
    fn build(&self, app: &mut crate::App) {
        let (sender, receiver) = crossbeam_channel::bounded::<HotPatched>(1);

        // Connects to the dioxus CLI that will handle rebuilds
        // On a successful rebuild the CLI sends a `HotReload` message with the new jump table
        // When receiving that message, update the table and send a `HotPatched` message through the channel
        connect(move |msg| {
            if let DevserverMsg::HotReload(hot_reload_msg) = msg {
                if let Some(jumptable) = hot_reload_msg.jump_table {
                    // SAFETY: This is not unsafe, but anything using the updated jump table is.
                    // The table must be built carefully
                    unsafe { apply_patch(jumptable).unwrap() };
                    sender.send(HotPatched).unwrap();
                }
            }
        });

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
