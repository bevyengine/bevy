//! Utilities for hotpatching code.
extern crate alloc;

use alloc::sync::Arc;

#[cfg(feature = "reflect_auto_register")]
use bevy_ecs::schedule::IntoScheduleConfigs;
use bevy_ecs::{
    change_detection::DetectChangesMut, event::EventWriter, system::ResMut, HotPatchChanges,
    HotPatched,
};
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
            sender.send(HotPatched).unwrap();
        }));

        // Adds a system that will read the channel for new `HotPatched` messages, send the event, and update change detection.
        app.init_resource::<HotPatchChanges>()
            .add_event::<HotPatched>()
            .add_systems(
                Last,
                move |mut events: EventWriter<HotPatched>, mut res: ResMut<HotPatchChanges>| {
                    if receiver.try_recv().is_ok() {
                        events.write_default();
                        res.set_changed();
                    }
                },
            );

        #[cfg(feature = "reflect_auto_register")]
        app.add_systems(
            crate::First,
            (move |registry: bevy_ecs::system::Res<bevy_ecs::reflect::AppTypeRegistry>| {
                registry.write().register_derived_types();
            })
            .run_if(bevy_ecs::schedule::common_conditions::on_event::<HotPatched>),
        );
    }
}
