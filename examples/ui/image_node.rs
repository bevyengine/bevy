//! This example illustrates the basic usage of an image node.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands.spawn((
        Node {
            width: percent(100),
            height: percent(100),
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        children![(
            ImageNode::new(asset_server.load("branding/icon.png")),
            Node {
                width: Val::Px(256.),
                height: Val::Px(256.),
                ..default()
            }
        )],
    ));
}
