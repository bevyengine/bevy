use bevy::{prelude::*, render::pass::ClearColor, winit::WinitConfig};

fn main() {
    println!("Running first App.");
    App::build()
        .add_resource(WinitConfig {
            return_from_run: true,
        })
        .add_resource(ClearColor(Color::rgb(0.2, 0.2, 0.8)))
        .add_plugins(DefaultPlugins)
        .add_system(system1.system())
        .run();
    println!("Running another App.");
    App::build()
        .add_resource(WinitConfig {
            return_from_run: true,
        })
        .add_resource(ClearColor(Color::rgb(0.2, 0.8, 0.2)))
        .add_plugins_with(DefaultPlugins, |group| {
            group.disable::<bevy::log::LogPlugin>()
        })
        .add_system(system2.system())
        .run();
    println!("Done.");
}

fn system1() {
    info!("logging from first app");
}

fn system2() {
    info!("logging from second app");
}
