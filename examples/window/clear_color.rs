use bevy::{prelude::*, render::pass::ClearColor};

fn main() {
    App::build()
        .insert_resource(ClearColor(Color::rgb(0.5, 0.5, 0.9)))
        .add_plugins(DefaultPlugins)
        .add_system(bevy::input::system::exit_on_esc_system.system())
        .run();
}
