//! An example of registering an extra asset source, and loading assets from it.
//! This asset source exists in addition to the default asset source.

use bevy::{
    asset::{
        io::{AssetSourceBuilder, AssetSourceId},
        AssetPath,
    },
    prelude::*,
};
use std::path::Path;

fn main() {
    App::new()
        // DefaultPlugins contains AssetPlugin so it must be added to our App
        // before inserting our new asset source.
        .add_plugins(DefaultPlugins)
        // Add an extra asset source with the name "example_files" to
        // AssetSourceBuilders.
        .register_asset_source(
            "example_files",
            AssetSourceBuilder::platform_default("examples/asset/files", None),
        )
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    // Now we can load the asset using our new asset source.
    //
    // The actual file path relative to workspace root is
    // "examples/asset/files/bevy_pixel_light.png".
    let path = Path::new("bevy_pixel_light.png");
    let source = AssetSourceId::from("example_files");
    let asset_path = AssetPath::from_path(path).with_source(source);

    // You could also parse this URL-like string representation for the asset
    // path.
    assert_eq!(asset_path, "example_files://bevy_pixel_light.png".into());

    commands.spawn(Sprite::from_image(asset_server.load(asset_path)));
}
