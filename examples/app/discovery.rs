use bevy::prelude::*;

#[derive(DiscoveryPlugin)]
#[root("examples/app/discovery.rs")]
struct DiscPlugin;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_plugin(DiscPlugin)
        .run();
}

#[system]
fn discovered_system() {
    println!("Woo, discovered system!");
}