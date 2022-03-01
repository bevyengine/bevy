use bevy::{prelude::*, render::settings::WgpuSettings};

fn main() {
    App::new()
        .insert_resource(WgpuSettings {
            backends: None,
            ..default()
        })
        .add_plugins(DefaultPlugins)
        .run();
}
