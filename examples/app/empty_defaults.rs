//! An empty application with default plugins.

use bevy::prelude::*;

fn main() {
    App::new().add_plugin_group(DefaultPlugins).run();
}
