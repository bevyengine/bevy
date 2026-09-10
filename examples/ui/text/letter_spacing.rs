//! This example demonstrates the `LetterSpacing` component in Bevy's text system.
//!
//! Use the left and right arrow keys to adjust the letter spacing of the text.

use bevy::prelude::*;
use bevy::text::{LetterSpacing, RemSize};

#[derive(Component)]
struct LetterSpacingLabel;

#[derive(Component)]
struct AnimatedLetterSpacing;

#[derive(Resource, Default)]
enum SpacingMode {
    #[default]
    Px,
    Rem,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .init_resource::<SpacingMode>()
        .add_systems(Startup, setup)
        .add_systems(Update, (update_letter_spacing, toggle_mode))
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    let font = asset_server.load("fonts/FiraSans-Bold.ttf");

    commands
        .spawn(Node {
            width: percent(100),
            height: percent(100),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(Node {
                    width: percent(100),
                    height: percent(100),
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(vw(5), vh(10)),
                    row_gap: vh(6),
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("HELLO"),
                        Underline,
                        TextFont {
                            font: font.clone().into(),
                            font_size: FontSize::Vh(6.0),
                            ..default()
                        },
                        Node {
                            padding: vh(2).bottom(),
                            ..default()
                        },
                    ));

                    // Left justified
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            width: percent(100.0),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Justify::Left"),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Vh(2.0),
                                    ..default()
                                },
                            ));
                            parent.spawn((
                                Text::new("letter spacing"),
                                AnimatedLetterSpacing,
                                TextLayout::justify(Justify::Left),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Vh(6.0),
                                    ..default()
                                },
                                Node {
                                    width: percent(100.0),
                                    ..default()
                                },
                                // Custom `LetterSpacing` can be added to any text entity as a component
                                LetterSpacing::Px(0.0),
                            ));
                        });

                    // Center justified
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            width: percent(100.0),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Justify::Center"),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Vh(2.0),
                                    ..default()
                                },
                            ));
                            parent.spawn((
                                Text::new("letter spacing"),
                                AnimatedLetterSpacing,
                                TextLayout::justify(Justify::Center),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Vh(6.0),
                                    ..default()
                                },
                                Node {
                                    width: percent(100.0),
                                    ..default()
                                },
                                // Custom `LetterSpacing` can be added to any text entity as a component
                                LetterSpacing::Px(0.0),
                            ));
                        });

                    // Right justified
                    parent
                        .spawn(Node {
                            flex_direction: FlexDirection::Column,
                            width: percent(100.0),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn((
                                Text::new("Justify::Right"),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Vh(2.0),
                                    ..default()
                                },
                            ));
                            parent.spawn((
                                Text::new("letter spacing"),
                                AnimatedLetterSpacing,
                                TextLayout::justify(Justify::Right),
                                TextFont {
                                    font: font.clone().into(),
                                    font_size: FontSize::Vh(6.0),
                                    ..default()
                                },
                                Node {
                                    width: percent(100.0),
                                    ..default()
                                },
                                // Custom `LetterSpacing` can be added to any text entity as a component
                                LetterSpacing::Px(0.0),
                            ));
                        });
                });

            parent.spawn((
                Text::new("LetterSpacing::Px(0.0)"),
                LetterSpacingLabel,
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Vh(3.0),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    bottom: vh(2.0),
                    left: vw(2.0),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("← → to adjust   Space to toggle Px / Rem"),
                TextFont {
                    font: font.clone().into(),
                    font_size: FontSize::Vh(2.5),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    bottom: vh(2.0),
                    right: vw(2.0),
                    ..default()
                },
            ));
        });
}

fn toggle_mode(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<SpacingMode>,
    rem_size: Res<RemSize>,
    mut query: Query<&mut LetterSpacing, With<AnimatedLetterSpacing>>,
    mut label_query: Query<&mut Text, With<LetterSpacingLabel>>,
) {
    if !keyboard.just_pressed(KeyCode::Space) {
        return;
    }

    for mut spacing in &mut query {
        let new_spacing = match *spacing {
            LetterSpacing::Px(v) => {
                *mode = SpacingMode::Rem;
                LetterSpacing::Rem(v / rem_size.0)
            }
            LetterSpacing::Rem(v) => {
                *mode = SpacingMode::Px;
                LetterSpacing::Px(v * rem_size.0)
            }
        };
        *spacing = new_spacing;
    }

    for mut text in &mut label_query {
        match *mode {
            SpacingMode::Px => {
                if let Some(LetterSpacing::Px(v)) = query.iter().next().copied() {
                    text.0 = format!("LetterSpacing::Px({:.1})", v);
                }
            }
            SpacingMode::Rem => {
                if let Some(LetterSpacing::Rem(v)) = query.iter().next().copied() {
                    text.0 = format!("LetterSpacing::Rem({:.2})", v);
                }
            }
        }
    }
}

fn update_letter_spacing(
    keyboard: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut LetterSpacing, With<AnimatedLetterSpacing>>,
    mut label_query: Query<&mut Text, With<LetterSpacingLabel>>,
) {
    let delta = if keyboard.pressed(KeyCode::ArrowRight) {
        0.5
    } else if keyboard.pressed(KeyCode::ArrowLeft) {
        -0.5
    } else {
        return;
    };

    for mut spacing in &mut query {
        match *spacing {
            LetterSpacing::Px(current) => {
                let new_value = (current + delta).clamp(-100.0, 100.0);
                *spacing = LetterSpacing::Px(new_value);
                for mut text in &mut label_query {
                    text.0 = format!("LetterSpacing::Px({:.1})", new_value);
                }
            }
            LetterSpacing::Rem(current) => {
                let new_value = (current + delta * 0.1).clamp(-10.0, 10.0);
                *spacing = LetterSpacing::Rem(new_value);
                for mut text in &mut label_query {
                    text.0 = format!("LetterSpacing::Rem({:.2})", new_value);
                }
            }
        }
    }
}
