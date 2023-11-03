//! A dynamic library that contains a dynamically loaded plugin. See the `load_dynamic_plugin` on how to load such a plugin.

use bevy::prelude::*;

/// Derive DynamicPlugin on one main plugin, which will then be loaded by [`bevy_dynamic_plugin::load_dynamic_plugin`].
#[derive(DynamicPlugin)]
struct MyDynamicPlugin;

impl Plugin for MyDynamicPlugin {
    fn build(&self, app: &mut App) {
        info!("Plugin is being loaded...");
        app.add_systems(Update, say_hello);
    }
}

fn say_hello() {
    info!("Hello from the dynamic plugin!");
}
