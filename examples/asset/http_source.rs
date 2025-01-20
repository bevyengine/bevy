//! Example usage of the `http` asset source to load assets from the web.
//!
//! Due to [licensing complexities](https://github.com/briansmith/ring/issues/1827)
//! secure `https` requests are disabled by default in non-wasm builds.
//! To enable add this to your dependencies in Cargo.toml:
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
