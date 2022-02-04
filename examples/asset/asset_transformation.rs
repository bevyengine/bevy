use bevy::{prelude::*, render::texture::TextureFormatPixelInfo};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup.system())
        .run();
}

fn filter_pixels(filter: usize, image: &Image) -> Vec<u8> {
    image
        .data
        .iter()
        .enumerate()
        .map(|(i, v)| {
            if i / image.texture_descriptor.format.pixel_size() % filter == 0 {
                0
            } else {
                *v
            }
        })
        .collect()
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    let texture_handle = asset_server.load("branding/icon.png");

    // new texture with every third pixel removed
    let texture_handle_1 = asset_server.create_from(texture_handle.clone(), |texture: &Image| {
        Some(Image {
            data: filter_pixels(3, texture),
            ..texture.clone()
        })
    });

    // new texture with every second pixel removed
    let texture_handle_2 = asset_server.create_from(texture_handle.clone(), |texture: &Image| {
        Some(Image {
            data: filter_pixels(2, texture),
            ..texture.clone()
        })
    });

    commands.spawn_bundle(OrthographicCameraBundle::new_2d());

    commands.spawn_bundle(SpriteBundle {
        texture: texture_handle,
        transform: Transform::from_xyz(-300.0, 0.0, 0.0),
        ..Default::default()
    });
    commands.spawn_bundle(SpriteBundle {
        texture: texture_handle_1,
        ..Default::default()
    });
    commands.spawn_bundle(SpriteBundle {
        texture: texture_handle_2,
        transform: Transform::from_xyz(300.0, 0.0, 0.0),
        ..Default::default()
    });
}
