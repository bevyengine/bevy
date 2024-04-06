//! This example demonstrates the usage of '.meta' files to override the default settings for loading an asset

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(
            // This just tells the asset server to look in the right examples folder
            DefaultPlugins.set(AssetPlugin {
                file_path: "examples/asset/files".to_string(),
                ..Default::default()
            }),
        )
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Without any .meta file specifying settings, the default sampler [ImagePlugin::default()] is used for loading images.
    // If you are using a very small image and rendering it larger like seen here, the default linear filtering will result in a blurry image.
    commands.spawn(SpriteBundle {
        texture: asset_server.load("bevy_pixel_dark.png"),
        sprite: Sprite {
            custom_size: Some(Vec2 { x: 160.0, y: 120.0 }),
            ..Default::default()
        },
        transform: Transform::from_xyz(-100.0, 0.0, 0.0),
        ..Default::default()
    });

    // When a .meta file is added with the same name as the asset and a '.meta' extension
    // you can (and must) specify all fields of the asset loader's settings for that
    // particular asset, in this case [ImageLoaderSettings]. Take a look at
    // examples/asset/files/bevy_pixel_dark_with_meta.png.meta
    // for the format and you'll notice, the only non-default option is setting Nearest
    // filtering. This tends to work much better for pixel art assets.
    // A good reference when filling this out is to check out [ImageLoaderSettings::default()]
    // and follow to the default implementation of each fields type.
    commands.spawn(SpriteBundle {
        texture: asset_server.load("bevy_pixel_dark_with_meta.png"),
        sprite: Sprite {
            custom_size: Some(Vec2 { x: 160.0, y: 120.0 }),
            ..Default::default()
        },
        transform: Transform::from_xyz(100.0, 0.0, 0.0),
        ..Default::default()
    });

    commands.spawn(Camera2dBundle::default());
}
