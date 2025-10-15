//! This example illustrates UI text with strikeout

use bevy::{
    color::palettes::css::{GREEN, NAVY, RED},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn((
        Text::new("struck\nstruck"),
        // Just add the `Strikeout` component to any `Text`, `Text2d` or `TextSpan` and it's text will be struck out.
        Strikeout,
        TextFont {
            font: asset_server.load("fonts/FiraSans-Bold.ttf"),
            font_size: 67.0,
            ..default()
        },
        TextLayout::new_with_justify(Justify::Center),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(5),
            right: px(5),
            ..default()
        },
        TextBackgroundColor::BLACK,
    ));

    commands.spawn((
        Node {
            flex_direction: FlexDirection::Column,
            width: Val::Percent(100.),
            height: Val::Percent(100.),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..Default::default()
        },
        children![
            (
                Text::new("struck\nstruckstruck\nstruckstuckstruck"),
                Strikeout,
                TextBackgroundColor(GREEN.into()),
            ),
            Text::new("normal"),
            (
                Text::new("struck"),
                Strikeout,
                TextBackgroundColor(GREEN.into()),
                children![
                    TextSpan::new("normal"),
                    (TextSpan::new("struck"), Strikeout,)
                ],
            ),
            (
                Text::new("struck struck"),
                Strikeout,
                TextFont {
                    font_size: 67.0,
                    ..default()
                },
            ),
            (
                Text::new("2struck\nstruck"),
                Strikeout,
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                    font_size: 67.0,
                    ..default()
                },
                BackgroundColor(NAVY.into())
            ),
            (
                Text::new(""),
                children![
                    (
                        TextSpan::new("struck"),
                        Strikeout,
                        TextFont {
                            font_size: 15.,
                            ..default()
                        },
                        TextColor(RED.into()),
                        TextBackgroundColor(Color::BLACK)
                    ),
                    (
                        TextSpan::new("\nnormal"),
                        TextFont {
                            font_size: 30.,
                            ..default()
                        },
                        TextColor(RED.into()),
                        TextBackgroundColor(GREEN.into())
                    ),
                    (
                        TextSpan::new("\nstruck"),
                        TextFont {
                            font_size: 50.,
                            ..default()
                        },
                        Strikeout,
                        TextColor(RED.into()),
                        TextBackgroundColor(NAVY.into())
                    ),
                    (
                        TextSpan::new("struck"),
                        TextFont {
                            font_size: 70.,
                            ..default()
                        },
                        Strikeout,
                        TextColor(RED.into()),
                        TextBackgroundColor(NAVY.into())
                    )
                ]
            ),
        ],
    ));
}
