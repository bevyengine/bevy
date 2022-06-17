//! An empty application with default plugins.

use bevy::prelude::*;

#[bevy_main]
async fn main() {
    App::new().add_plugins(DefaultPlugins).await.run();
}
