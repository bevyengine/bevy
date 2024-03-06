//! An example of registering an extra asset source, and loading assets from it.
//! This asset source exists in addition to the default asset source.

use bevy::asset::{
    io::{AssetSourceBuilder, AssetSourceBuilders, AssetSourceId},
    AssetPath,
};
use bevy::prelude::*;
use std::path::Path;

fn main() {
    let mut app = App::new();

    // We add an extra asset source with the name "example_files" to the
    // AssetSourceBuilders.
    // This needs to be done before AssetPlugin finalizes building them
    let mut sources = app
        .world
        .get_resource_or_insert_with::<AssetSourceBuilders>(default);
    sources.insert(
        "example_files",
        AssetSourceBuilder::platform_default("examples/asset/files", None),
    );

    // DefaultPlugins contains AssetPlugin so it needs to be added to our App
    // after inserting our new asset source
    app.add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    // Now we can load the asset using our new asset source
    //
    // The actual file path relative to workspace root is
    // "examples/asset/files/bevy_pixel_light.png".
    let path = Path::new("bevy_pixel_light.png");
    let source = AssetSourceId::from("example_files");
    let asset_path = AssetPath::from_path(path).with_source(source);

    // You could also parse this URL-like string representation for the asset
    // path.
    assert_eq!(asset_path, "example_files://bevy_pixel_light.png".into());

    commands.spawn(SpriteBundle {
        texture: asset_server.load(asset_path),
        ..default()
    });
}
