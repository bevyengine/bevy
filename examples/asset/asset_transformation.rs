use bevy::prelude::*;

fn main() {
    App::build()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server.load("branding/icon.png");
    // new texture with one pixel every three removed
    let texture_handle_1 =
        asset_server.create_new_from(texture_handle.clone(), |texture: &Texture| {
            let new_data = texture
                .data
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    if i / texture.format.pixel_size() % 3 == 0 {
                        0
                    } else {
                        *v
                    }
                })
                .collect();
            Texture {
                data: new_data,
                ..*texture
            }
        });
    // new texture with one pixel every two removed
    let texture_handle_2 =
        asset_server.create_new_from(texture_handle.clone(), |texture: &Texture| {
            let new_data = texture
                .data
                .iter()
                .enumerate()
                .map(|(i, v)| {
                    if i / texture.format.pixel_size() % 2 == 0 {
                        0
                    } else {
                        *v
                    }
                })
                .collect();
            Texture {
                data: new_data,
                ..*texture
            }
        });

    commands
        .spawn(OrthographicCameraBundle::new_2d())
        .spawn(SpriteBundle {
            material: materials.add(texture_handle.into()),
            transform: Transform::from_xyz(-300.0, 0.0, 0.0),
            ..Default::default()
        })
        .spawn(SpriteBundle {
            material: materials.add(texture_handle_1.into()),
            ..Default::default()
        })
        .spawn(SpriteBundle {
            material: materials.add(texture_handle_2.into()),
            transform: Transform::from_xyz(300.0, 0.0, 0.0),
            ..Default::default()
        });
}
