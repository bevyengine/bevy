//! Example of loading an embedded asset.

use bevy::asset::{embedded_asset, io::AssetSourceId, AssetPath};
use bevy::prelude::*;
use std::path::Path;

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, EmbeddedAssetPlugin))
        .add_systems(Startup, setup)
        .run();
}

struct EmbeddedAssetPlugin;

impl Plugin for EmbeddedAssetPlugin {
    fn build(&self, app: &mut App) {
        // We get to choose some prefix relative to the workspace root which
        // will be ignored in "embedded://" asset paths.
        let omit_prefix = "examples/asset";
        // Path to asset must be relative to this file, because that's how
        // include_bytes! works.
        embedded_asset!(app, omit_prefix, "files/bevy_pixel_light.png");
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());

    // Each example is its own crate (with name from [[example]] in Cargo.toml).
    let crate_name = "embedded_asset";

    // The actual file path relative to workspace root is
    // "examples/asset/files/bevy_pixel_light.png".
    //
    // We omit the "examples/asset" from the embedded_asset! call and replace it
    // with the crate name.
    let path = Path::new(crate_name).join("files/bevy_pixel_light.png");
    let source = AssetSourceId::from("embedded");
    let asset_path = AssetPath::from_path(&path).with_source(source);

    // You could also parse this URL-like string representation for the asset
    // path.
    assert_eq!(
        asset_path,
        "embedded://embedded_asset/files/bevy_pixel_light.png".into()
    );

    commands.spawn(SpriteBundle {
        texture: asset_server.load(asset_path),
        ..default()
    });
}
