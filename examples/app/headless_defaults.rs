use bevy::{prelude::*, render::options::WgpuOptions};

fn main() {
    App::new()
        .insert_setup_resource(WgpuOptions {
            backends: None,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .run();
}
