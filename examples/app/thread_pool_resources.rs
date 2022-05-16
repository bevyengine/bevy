use bevy::prelude::*;
use bevy::tasks::TaskPoolBuilder;

/// This example illustrates how to customize the thread pool used internally (e.g. to only use a
/// certain number of threads).
fn main() {
    App::new()
        .insert_resource(
            TaskPoolBuilder {
                min_total_threads: 4,
                max_total_threads: 16,
                ..Default::default()
            }
            .build(),
        )
        .add_plugins(DefaultPlugins)
        .run();
}
