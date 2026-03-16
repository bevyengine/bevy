//! This example demonstrates the `LetterSpacing` component in Bevy's text system.
//!
//! Use the left and right arrow keys to adjust the letter spacing of the text.

use bevy::prelude::*;
use bevy::text::LetterSpacing;

#[derive(Component)]
struct LetterSpacingLabel;

#[derive(Component)]
struct AnimatedLetterSpacing;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, update_letter_spacing)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);

    commands
        .spawn(Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(Node {
                    width: Val::Percent(100.0),
                    height: Val::Percent(100.0),
                    align_items: AlignItems::Center,
                    padding: UiRect::axes(Val::Vw(5.0), Val::Vh(10.0)),
                    flex_direction: FlexDirection::Column,
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn((
                        Text::new("HELLO"),
                        Underline,
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Vh(15.0),
                            ..default()
                        },
                        Node {
                            padding: UiRect::bottom(Val::Vh(2.0)),
                            ..default()
                        },
                    ));

                    parent.spawn((
                        Text::new("letter spacing"),
                        AnimatedLetterSpacing,
                        TextLayout::new_with_justify(Justify::Center),
                        TextFont {
                            font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                            font_size: FontSize::Vh(10.0),
                            ..default()
                        },
                        Node {
                            width: Val::Percent(100.0),
                            ..default()
                        },
                        LetterSpacing::Px(0.0),
                    ));
                });

            parent.spawn((
                Text::new("LetterSpacing::Px(0.0)"),
                LetterSpacingLabel,
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                    font_size: FontSize::Vh(3.0),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Vh(2.0),
                    left: Val::Vw(2.0),
                    ..default()
                },
            ));

            parent.spawn((
                Text::new("← → to adjust letter spacing"),
                TextFont {
                    font: asset_server.load("fonts/FiraSans-Bold.ttf").into(),
                    font_size: FontSize::Vh(2.5),
                    ..default()
                },
                Node {
                    position_type: PositionType::Absolute,
                    bottom: Val::Vh(2.0),
                    right: Val::Vw(2.0),
                    ..default()
                },
            ));
        });
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
        let LetterSpacing::Px(current) = *spacing;
        let new_value = (current + delta).clamp(-100.0, 100.0);
        *spacing = LetterSpacing::Px(new_value);

        for mut text in &mut label_query {
            text.0 = format!("LetterSpacing::Px({:.1})", new_value);
        }
    }
}
