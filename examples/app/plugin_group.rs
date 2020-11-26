use bevy::{app::PluginGroupBuilder, prelude::*};

/// PluginGroups are a way to group sets of plugins that should be registered together.
fn main() {
    App::build()
        // Two PluginGroups that are included with bevy are DefaultPlugins and MinimalPlugins
        .add_plugins(DefaultPlugins)
        // Adding a plugin group adds all plugins in the group by default
        .add_plugins(HelloWorldPlugins)
        // You can also modify a PluginGroup (such as disabling plugins) like this:
        // .add_plugins_with(HelloWorldPlugins, |group| {
        //     group
        //         .disable::<PrintWorldPlugin>()
        //         .add_before::<PrintHelloPlugin, _>(bevy::diagnostic::PrintDiagnosticsPlugin::default())
        // })
        .run();
}

/// A group of plugins that produce the "hello world" behavior
pub struct HelloWorldPlugins;

impl PluginGroup for HelloWorldPlugins {
    fn build(&mut self, group: &mut PluginGroupBuilder) {
        group.add(PrintHelloPlugin).add(PrintWorldPlugin);
    }
}

pub struct PrintHelloPlugin;

impl Plugin for PrintHelloPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(print_hello_system);
    }
}

fn print_hello_system() {
    println!("hello");
}

pub struct PrintWorldPlugin;

impl Plugin for PrintWorldPlugin {
    fn build(&self, app: &mut AppBuilder) {
        app.add_system(print_world_system);
    }
}

fn print_world_system() {
    println!("world");
}
