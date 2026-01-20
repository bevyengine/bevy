//! This example illustrates the basic usage of an `ImageNode`.
//! `ImageNode` is UI Node that render an Image.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // Ui camera
    commands.spawn(Camera2d);

    commands.spawn((
        // This root Node serves as a container for the ImageNode.
        // In this case, it will center the item on the screen.
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        // Child Nodes are added with the `children!` macro.
        children![(
            // Create a new `ImageNode` with the given texture.
            ImageNode::new(asset_server.load("branding/icon.png")),
            // Child Node control `ImageNode` size
            Node {
                width: Val::Px(256.),
                height: Val::Px(256.),
                ..default()
            }
        )],
    ));
}
