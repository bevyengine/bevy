use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    command_buffer: &mut CommandBuffer,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("assets/branding/icon.png").unwrap();
    command_buffer
        .build()
        .add_entity(OrthographicCameraEntity::default())
        .add_entity(SpriteEntity {
            rect: Rect {
                position: Vec2::new(300.0, 300.0),
                z_index: 0.5,
                ..Default::default()
            },
            material: materials.add(texture_handle.into()),
            ..Default::default()
        });
}
