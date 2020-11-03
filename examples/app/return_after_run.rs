use bevy::{prelude::*, render::pass::ClearColor, winit::WinitConfig};

fn main() {
    println!("Running first App.");
    App::build()
        .add_resource(WinitConfig {
            return_from_run: true,
        })
        .add_resource(ClearColor(Color::rgb(0.2, 0.2, 0.8)))
        .add_plugins(DefaultPlugins)
        .run();
    println!("Running another App.");
    App::build()
        .add_resource(WinitConfig {
            return_from_run: true,
        })
        .add_resource(ClearColor(Color::rgb(0.2, 0.8, 0.2)))
        .add_plugins(DefaultPlugins)
        .run();
    println!("Done.");
}
