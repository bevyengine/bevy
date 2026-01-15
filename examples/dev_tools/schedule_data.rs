//! This example demonstrates how to automatically serialize schedule data.

use bevy::{dev_tools::schedule_data::plugin::*, prelude::*};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, SerializeSchedulesPlugin::default()))
        // This resource is only necessary to put the output in a nice spot for the example code.
        .insert_resource(SerializeSchedulesFilePath(
            "examples/dev_tools/app_data.ron".into(),
        ))
        .run();
}
