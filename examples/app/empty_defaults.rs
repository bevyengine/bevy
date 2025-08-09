//! An empty application with default plugins.

use bevy::prelude::*;

fn main() {
    App::default().add_plugins(DefaultPlugins).run();
}
