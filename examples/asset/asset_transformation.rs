use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .run();
}

fn filter_pixels(filter: usize, texture: &Texture) -> Vec<u8> {
    texture
        .data
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i / texture.format.pixel_size() % filter == 0 {
                0
            } else {
                *v
            }
        })
        .collect()
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("branding/icon.png");

    // new texture with every third pixel removed
    let texture_handle_1 = asset_server.create_from(texture_handle.clone(), |texture: &Texture| {
        Some(Texture {
            data: filter_pixels(3, texture),
            ..*texture
        })
    });

    // new texture with every second pixel removed
    let texture_handle_2 = asset_server.create_from(texture_handle.clone(), |texture: &Texture| {
        Some(Texture {
            data: filter_pixels(2, texture),
            ..*texture
        })
    });

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands.spawn_bundle(SpriteBundle {
        material: materials.add(texture_handle.into()),
        transform: Transform::from_xyz(-300.0, 0.0, 0.0),
        ..Default::default()
    });
    commands.spawn_bundle(SpriteBundle {
        material: materials.add(texture_handle_1.into()),
        ..Default::default()
    });
    commands.spawn_bundle(SpriteBundle {
        material: materials.add(texture_handle_2.into()),
        transform: Transform::from_xyz(300.0, 0.0, 0.0),
        ..Default::default()
    });
}
