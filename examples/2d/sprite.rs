use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
    command_buffer: &mut CommandBuffer,
) {
    let texture_handle = asset_server.load("assets/branding/icon.png").unwrap();
    command_buffer
        .build()
        .entity_with(OrthographicCameraComponents::default())
        .entity_with(SpriteComponents {
            material: materials.add(texture_handle.into()),
            ..Default::default()
        });
}
