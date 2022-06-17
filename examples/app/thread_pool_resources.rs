//! This example illustrates how to customize the thread pool used internally (e.g. to only use a
//! certain number of threads).

use bevy::prelude::*;

#[bevy_main]
async fn main() {
    App::new()
        .insert_resource(DefaultTaskPoolOptions::with_num_threads(4))
        .add_plugins(DefaultPlugins)
        .await
        .run();
}
