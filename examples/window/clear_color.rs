use bevy::{prelude::*, render::pass::ClearColor};

fn main() {
    App::build()
        .add_resource(ClearColor(Color::rgb(0.5, 0.5, 0.9)))
        .add_plugins(DefaultPlugins)
        .run();
}
