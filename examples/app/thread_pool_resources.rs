//! This example illustrates how to customize the thread pool used internally (e.g. to only use a
//! certain number of threads).

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(TaskPoolPlugin {
            task_pool_options: TaskPoolOptions::with_num_threads(4),
        }))
        .run();
}
