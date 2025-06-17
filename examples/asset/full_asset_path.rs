//! Shows how to get the full path of an asset that was loaded from disk.

use bevy::{asset::io::FullAssetPathProvider, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_observer(print_full_asset_path)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands.spawn((
        Sprite::from_image(asset_server.load("branding/icon.png")),
        Transform::from_xyz(-300., 0., 0.),
    ));

    commands.spawn((
        Sprite::from_image(asset_server.load("textures/Game Icons/wrench.png")),
        Transform::from_xyz(0., 0., 0.),
    ));

    commands.spawn((
        Sprite::from_image(asset_server.load("textures/Game Icons/right.png")),
        Transform::from_xyz(300., 0., 0.),
    ));
}

/// Every time a sprite is added, we print the full asset path.
/// We use `Sprite` here, which contains a `Handle<Image>`, but it works with any kind of `Handle<T>`.
fn print_full_asset_path(
    trigger: On<Add, Sprite>,
    sprites: Query<&Sprite>,
    full_path_provider: Res<FullAssetPathProvider>,
) {
    let entity = trigger.target();
    let Ok(sprite) = sprites.get(entity) else {
        return;
    };
    let Some(asset_path) = sprite.image.path() else {
        error!("The loaded sprite has no asset path");
        return;
    };
    match full_path_provider.full_asset_path(&asset_path) {
        Ok(full_path) => info!("Full asset path: {:?}", full_path),
        Err(e) => error!(
            "Failed to get full asset path for {}: {}",
            asset_path.path().display(),
            e
        ),
    }
}
