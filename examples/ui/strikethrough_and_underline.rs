//! This example illustrates UI text with strikethrough and underline decorations

use bevy::{
    color::palettes::css::{GREEN, NAVY, RED, YELLOW},
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
        // Just add the `Strikethrough` component to any `Text`, `Text2d` or `TextSpan` and its text will be struck through
        Strikethrough,
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
                Strikethrough,
                StrikethroughColor(RED.into()),
                TextBackgroundColor(GREEN.into()),
            ),
            // Text entities with the `Underline` component will drawn with underline
            (Text::new("underline"), Underline),
            (
                Text::new("struck"),
                Strikethrough,
                TextBackgroundColor(GREEN.into()),
                children![
                    (TextSpan::new("underline"), Underline),
                    (TextSpan::new("struck"), Strikethrough,)
                ],
            ),
            (
                Text::new("struck struck"),
                Strikethrough,
                TextFont {
                    font_size: 67.0,
                    ..default()
                },
            ),
            (
                Text::new("2struck\nstruck"),
                Strikethrough,
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
                        Strikethrough,
                        TextFont {
                            font_size: 15.,
                            ..default()
                        },
                        TextColor(RED.into()),
                        TextBackgroundColor(Color::BLACK)
                    ),
                    (
                        TextSpan::new("\nunderline"),
                        Underline,
                        UnderlineColor(YELLOW.into()),
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
                        Strikethrough,
                        TextColor(RED.into()),
                        TextBackgroundColor(NAVY.into())
                    ),
                    (
                        TextSpan::new("underlined and struck"),
                        TextFont {
                            font_size: 70.,
                            ..default()
                        },
                        Strikethrough,
                        Underline,
                        TextColor(RED.into()),
                        TextBackgroundColor(NAVY.into()),
                        StrikethroughColor(Color::WHITE),
                        UnderlineColor(Color::WHITE),
                    )
                ]
            ),
        ],
    ));
}
