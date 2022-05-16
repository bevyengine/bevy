//! This example illustrates how to customize the thread pool used internally (e.g. to only use a
//! certain number of threads).

use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;

fn main() {
    App::new()
        .insert_resource(TaskPoolBuilder::new().threads(4).build())
        .add_plugins(DefaultPlugins)
        .run();
}
