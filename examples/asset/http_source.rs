//! Example usage of the `http` asset source to load assets from the web.
/// Note that the use of secure `https` sources in non-wasm builds requires the following dependency:
//! ```toml
//! ureq = { version = "*", features = ["tls"] }
//! ```
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands.spawn(
        // Simply use a url where you would normally use an asset folder relative path
        Sprite::from_image(asset_server.load("https://raw.githubusercontent.com/bevyengine/bevy/refs/heads/main/assets/branding/bevy_bird_dark.png"))
    );
}
