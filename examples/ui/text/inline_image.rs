//! Demonstrates how to display images inline with text.

use bevy::prelude::*;

fn main() {
    let mut app = App::new();
    app.add_plugins(DefaultPlugins).add_systems(Startup, setup);
    app.run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new("[Text]"),
        Node {
            margin: auto().all(),
            ..default()
        },
        children![
            (TextSpan::new("[span before image]"),),
            (InlineImage {
                color: bevy::color::palettes::css::RED.into(),
                image: asset_server.load("branding/bevy_logo_dark.png"),
            },),
            (TextSpan::new("[span between images]"),),
            (InlineImage {
                color: bevy::color::palettes::css::YELLOW.into(),
                image: asset_server.load("branding/bevy_logo_dark.png"),
            },),
            (TextSpan::new("[span after image]"),),
        ],
    ));
}
