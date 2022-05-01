//! An empty application with default plugins.

use bevy::prelude::*;

fn main() {
    App::new().add_plugins(DefaultPlugins).run();
}
