use bevy::{ecs::ParallelExecutorOptions, prelude::*};
use std::time::Duration;

/// This example illustrates how to customize the thread pool used internally (e.g. to only use a
/// certain number of threads).
fn main() {
    App::build()
        .add_resource(ParallelExecutorOptions::new().with_num_threads(Some(4)))
        .add_default_plugins()
        .run();
}
