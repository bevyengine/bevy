use bevy::{prelude::*, render::options::WgpuSettings};

fn main() {
    App::new()
        .insert_resource(WgpuSettings {
            backends: None,
            ..Default::default()
        })
        .add_plugins(DefaultPlugins)
        .run();
}
