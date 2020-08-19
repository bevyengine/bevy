use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("assets/branding/icon.png").unwrap();

    let sprite_material: ColorMaterial = texture_handle.into();
    // You can flip sprites by using:
    // sprite_material.flip_horz = 1.0;
    // sprite_material.flip_vert = 1.0;

    commands
        .spawn(Camera2dComponents::default())
        .spawn(SpriteComponents {
            material: materials.add(sprite_material),
            ..Default::default()
        });
}
