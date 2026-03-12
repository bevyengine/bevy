//! This example illustrates the basic usage of [`ImageNode`],
//! a UI node that renders an image.
//!
//! It also demonstrates how to use [`ImageNode::with_rect`] to render
//! only a sub-region of an image, which is an easy one-off alternative
//! to using a [`TextureAtlas`].

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let image = asset_server.load("branding/icon.png");

    commands.spawn((
        // Root node that centers everything on screen.
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            column_gap: px(24.),
            ..default()
        },
        children![
            // Full image.
            (
                ImageNode::new(image.clone()),
                Node {
                    width: px(256.),
                    height: px(256.),
                    ..default()
                },
            ),
            // Sub-region of the same image using `with_rect`.
            // This renders only the specified rectangle (in pixels)
            // from the source texture.
            (
                ImageNode::new(image).with_rect(Rect::new(0., 0., 128., 128.)),
                Node {
                    width: px(128.),
                    height: px(128.),
                    ..default()
                },
            ),
        ],
    ));
}
