//! This example illustrates how to customize the thread pool used internally (e.g. to only use a
//! certain number of threads).

use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(CorePlugin {
            task_pool_builder: TaskPoolBuilder::new().threads(4),
        }))
        .run();
}
