//! This example demonstrates the usage of '.meta' files and [`AssetServer::load_with_settings`] to override the default settings for loading an asset

use bevy::{
    prelude::*,
    render::texture::{ImageLoaderSettings, ImageSampler},
};

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
    // Useful note: The default sampler specified by the ImagePlugin is *not* the same as the default implementation of sampler. This is why
    // everything uses linear by default but if you look at the default of sampler, it uses nearest.
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
    // https://docs.rs/bevy/latest/bevy/render/texture/struct.ImageLoaderSettings.html#
    commands.spawn(SpriteBundle {
        texture: asset_server.load("bevy_pixel_dark_with_meta.png"),
        sprite: Sprite {
            custom_size: Some(Vec2 { x: 160.0, y: 120.0 }),
            ..Default::default()
        },
        transform: Transform::from_xyz(100.0, 0.0, 0.0),
        ..Default::default()
    });

    // Another option is to use the AssetServers load_with_settings function.
    // With this you can specify the same settings upon loading your asset with a
    // couple of differences. A big one is that you aren't required to set *every*
    // setting, just modify the ones that you need. It works by passing in a function
    // (in this case an anonymous closure) that takes a reference to the settings type
    // that is then modified in the function.
    // Do note that if you want to load the same asset with different settings, the
    // settings changes from any loads after the first of the same asset will be ignored.
    // This is why this one loads a differently named copy of the asset instead of using
    // same one as without a .meta file.
    commands.spawn(SpriteBundle {
        texture: asset_server.load_with_settings(
            "bevy_pixel_dark_with_settings.png",
            |settings: &mut ImageLoaderSettings| {
                settings.sampler = ImageSampler::nearest();
            },
        ),
        sprite: Sprite {
            custom_size: Some(Vec2 { x: 160.0, y: 120.0 }),
            ..Default::default()
        },
        transform: Transform::from_xyz(0.0, 150.0, 0.0),
        ..Default::default()
    });

    commands.spawn(Camera2dBundle::default());
}
