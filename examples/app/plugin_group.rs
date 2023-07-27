//! Demonstrates the creation and registration of a custom plugin group.
//! [`PluginGroup`]s are a way to group sets of plugins that should be registered together.

use bevy::{app::PluginGroupBuilder, prelude::*};

fn main() {
    App::new()
        .add_plugins((
            // Two PluginGroups that are included with bevy are DefaultPlugins and MinimalPlugins
            DefaultPlugins,
            // Adding a plugin group adds all plugins in the group by default
            HelloWorldPlugins,
        ))
        // You can also modify a PluginGroup (such as disabling plugins) like this:
        // .add_plugins(
        //     HelloWorldPlugins
        //         .build()
        //         .disable::<PrintWorldPlugin>()
        //         .add_before::<PrintHelloPlugin, _>(
        //             bevy::diagnostic::LogDiagnosticsPlugin::default(),
        //         ),
        // )
        .run();
}

/// A group of plugins that produce the "hello world" behavior
pub struct HelloWorldPlugins;

impl PluginGroup for HelloWorldPlugins {
    fn build(self) -> PluginGroupBuilder {
        PluginGroupBuilder::start::<Self>()
            .add(PrintHelloPlugin)
            .add(PrintWorldPlugin)
    }
}

pub struct PrintHelloPlugin;

impl Plugin for PrintHelloPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, print_hello_system);
    }
}

fn print_hello_system() {
    info!("hello");
}

pub struct PrintWorldPlugin;

impl Plugin for PrintWorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, print_world_system);
    }
}

fn print_world_system() {
    info!("world");
}
