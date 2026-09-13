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
        TextFont::from_font_size(px(40.)),
        Node {
            margin: px(25).all(),
            justify_self: JustifySelf::Center,
            align_self: AlignSelf::Center,
            ..default()
        },
        BackgroundColor(bevy::color::palettes::css::MAROON.into()),
        Outline {
            width: px(2),
            ..default()
        },
        children![
            (
                TextSpan::new("[span before image]"),
                TextFont::from_font_size(px(40)),
            ),
            (InlineImage {
                color: bevy::color::palettes::css::RED.into(),
                image: asset_server.load("branding/bevy_logo_dark.png"),
                height: Some(30.),
                ..default()
            },),
            (
                TextSpan::new("[span between images]"),
                TextFont::from_font_size(px(40.)),
            ),
            (InlineImage {
                color: bevy::color::palettes::css::YELLOW.into(),
                image: asset_server.load("branding/bevy_bird_dark.png"),
                width: Some(30.),
                ..default()
            },),
            (
                TextSpan::new("[span after image]"),
                TextFont::from_font_size(px(40.)),
            ),
        ],
    ));
}
