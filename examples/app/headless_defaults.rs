use bevy::{prelude::*, render::options::WgpuOptions};

fn main() {
    App::new()
        .insert_initialization_resource(WgpuOptions {
            backends: None,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .run();
}
