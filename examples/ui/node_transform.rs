//! An example demonstrating how to translate, rotate and scale UI elements.

use bevy::prelude::*;

#[derive(Resource)]
struct Center(Entity);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, button_system)
        .add_systems(Update, translation_system)
        .run();
}

const NORMAL_BUTTON: Color = Color::WHITE;
const HOVERED_BUTTON: Color = Color::YELLOW;
const PRESSED_BUTTON: Color = Color::RED;

/// A button that rotates the target node
#[derive(Component)]
pub struct RotateButton(pub f32);

/// A button that scales the target node
#[derive(Component)]
pub struct ScaleButton(pub f32);

/// Marker component so the systems know which entities to translate, rotate and scale
#[derive(Component)]
pub struct TargetNode;

/// Handles button interactions
fn button_system(
    mut interaction_query: Query<
        (
            &Interaction,
            &mut BackgroundColor,
            Option<&RotateButton>,
            Option<&ScaleButton>,
        ),
        (Changed<Interaction>, With<Button>),
    >,
    mut rotator_query: Query<(&mut NodeRotation, &mut NodeScale), With<TargetNode>>,
) {
    for (interaction, mut color, maybe_rotate, maybe_scale) in &mut interaction_query {
        match *interaction {
            Interaction::Clicked => {
                *color = PRESSED_BUTTON.into();
                if let Some(step) = maybe_rotate {
                    for (mut rotation, ..) in rotator_query.iter_mut() {
                        rotation.0 += step.0;
                    }
                }
                if let Some(step) = maybe_scale {
                    for (_, mut scaling) in rotator_query.iter_mut() {
                        scaling.0 += step.0;
                        scaling.0 = scaling.0.clamp(Vec2::splat(0.25), Vec2::splat(3.0));
                    }
                }
            }
            Interaction::Hovered => {
                *color = HOVERED_BUTTON.into();
            }
            Interaction::None => {
                *color = NORMAL_BUTTON.into();
            }
        }
    }
}

// move the rotating panel when the arrow keys are pressed
fn translation_system(
    time: Res<Time>,
    input: Res<Input<KeyCode>>,
    mut translation_query: Query<&mut NodeTranslation, With<TargetNode>>,
) {
    let controls = [
        (KeyCode::Left, -Vec2::X),
        (KeyCode::Right, Vec2::X),
        (KeyCode::Up, -Vec2::Y),
        (KeyCode::Down, Vec2::Y),
    ];
    for &(key_code, direction) in &controls {
        if input.pressed(key_code) {
            for mut translation in translation_query.iter_mut() {
                translation.0 += direction * 50.0 * time.delta_seconds();
                translation.0 = translation.0.clamp(Vec2::splat(-150.), Vec2::splat(150.));
            }
        }
    }
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    // ui camera
    commands.spawn(Camera2dBundle::default());
    commands
        .spawn(NodeBundle {
            style: Style {
                flex_basis: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            background_color: Color::BLACK.into(),
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::SpaceEvenly,
                        gap: Size::all(Val::Px(25.)),
                        ..default()
                    },
                    ..default()
                })
                .with_children(|parent| {
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                padding: UiRect::all(Val::Px(10.)),
                                gap: Size::all(Val::Px(10.)),
                                ..default()
                            },
                            background_color: Color::BLACK.into(),
                            z_index: ZIndex::Global(1),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent
                                .spawn(ButtonBundle {
                                    style: Style {
                                        size: Size::all(Val::Px(50.)),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    background_color: Color::WHITE.into(),
                                    ..default()
                                })
                                .insert(RotateButton(-std::f32::consts::FRAC_PI_8))
                                .with_children(|parent| {
                                    parent.spawn(TextBundle {
                                        text: Text::from_section(
                                            "<--",
                                            TextStyle {
                                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                                font_size: 16.0,
                                                color: Color::BLACK,
                                            },
                                        ),
                                        ..default()
                                    });
                                });

                            parent
                                .spawn(ButtonBundle {
                                    style: Style {
                                        size: Size::all(Val::Px(50.)),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    background_color: Color::WHITE.into(),
                                    ..default()
                                })
                                .insert(ScaleButton(-0.25))
                                .with_children(|parent| {
                                    parent.spawn(TextBundle {
                                        text: Text::from_section(
                                            "-",
                                            TextStyle {
                                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                                font_size: 16.0,
                                                color: Color::BLACK,
                                            },
                                        ),
                                        ..default()
                                    });
                                });
                        });

                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::SpaceBetween,
                                align_items: AlignItems::Center,
                                size: Size::all(Val::Px(300.)),
                                ..default()
                            },
                            background_color: Color::DARK_GRAY.into(),
                            ..default()
                        })
                        .insert(TargetNode)
                        .insert(NodeRotation::default())
                        .insert(NodeScale::default())
                        .insert(NodeTranslation::default())
                        .with_children(|parent| {
                            parent
                                .spawn(ButtonBundle {
                                    style: Style {
                                        size: Size::all(Val::Px(50.)),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    background_color: Color::WHITE.into(),
                                    ..default()
                                })
                                .with_children(|parent| {
                                    parent.spawn(TextBundle {
                                        text: Text::from_section(
                                            "Top",
                                            TextStyle {
                                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                                font_size: 16.0,
                                                color: Color::BLACK,
                                            },
                                        ),
                                        ..default()
                                    });
                                });

                            parent
                                .spawn(NodeBundle {
                                    style: Style {
                                        align_self: AlignSelf::Stretch,
                                        justify_content: JustifyContent::SpaceBetween,
                                        ..default()
                                    },
                                    ..default()
                                })
                                .with_children(|parent| {
                                    parent
                                        .spawn(ButtonBundle {
                                            style: Style {
                                                size: Size::all(Val::Px(50.)),
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::Center,
                                                ..default()
                                            },
                                            background_color: Color::WHITE.into(),
                                            ..default()
                                        })
                                        .insert(NodeRotation(std::f32::consts::PI / 2.))
                                        .with_children(|parent| {
                                            parent.spawn(TextBundle {
                                                text: Text::from_section(
                                                    "Left",
                                                    TextStyle {
                                                        font: asset_server
                                                            .load("fonts/FiraSans-Bold.ttf"),
                                                        font_size: 16.0,
                                                        color: Color::BLACK,
                                                    },
                                                ),
                                                ..default()
                                            });
                                        });

                                    parent
                                        .spawn(ButtonBundle {
                                            style: Style {
                                                size: Size::all(Val::Px(50.)),
                                                align_items: AlignItems::Center,
                                                justify_content: JustifyContent::Center,
                                                ..default()
                                            },
                                            background_color: Color::WHITE.into(),
                                            ..default()
                                        })
                                        .insert(NodeRotation(-std::f32::consts::PI / 2.))
                                        .with_children(|parent| {
                                            parent.spawn(TextBundle {
                                                text: Text::from_section(
                                                    "Right",
                                                    TextStyle {
                                                        font: asset_server
                                                            .load("fonts/FiraSans-Bold.ttf"),
                                                        font_size: 16.0,
                                                        color: Color::BLACK,
                                                    },
                                                ),
                                                ..default()
                                            });
                                        });
                                });

                            parent
                                .spawn(ButtonBundle {
                                    style: Style {
                                        size: Size::all(Val::Px(50.)),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    background_color: Color::WHITE.into(),
                                    ..default()
                                })
                                .insert(NodeRotation(std::f32::consts::PI))
                                .with_children(|parent| {
                                    parent.spawn(TextBundle {
                                        text: Text::from_section(
                                            "Bottom",
                                            TextStyle {
                                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                                font_size: 16.0,
                                                color: Color::BLACK,
                                            },
                                        ),
                                        ..default()
                                    });
                                });
                        });
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::Center,
                                gap: Size::all(Val::Px(10.)),
                                padding: UiRect::all(Val::Px(10.)),
                                ..default()
                            },
                            background_color: Color::BLACK.into(),
                            z_index: ZIndex::Global(1),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent
                                .spawn(ButtonBundle {
                                    style: Style {
                                        size: Size::all(Val::Px(50.)),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    background_color: Color::WHITE.into(),
                                    ..default()
                                })
                                .insert(RotateButton(std::f32::consts::PI / 8.))
                                .with_children(|parent| {
                                    parent.spawn(TextBundle {
                                        text: Text::from_section(
                                            "-->",
                                            TextStyle {
                                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                                font_size: 16.0,
                                                color: Color::BLACK,
                                            },
                                        ),
                                        ..default()
                                    });
                                });

                            parent
                                .spawn(ButtonBundle {
                                    style: Style {
                                        size: Size::all(Val::Px(50.)),
                                        align_items: AlignItems::Center,
                                        justify_content: JustifyContent::Center,
                                        ..default()
                                    },
                                    background_color: Color::WHITE.into(),
                                    ..default()
                                })
                                .insert(ScaleButton(0.25))
                                .with_children(|parent| {
                                    parent.spawn(TextBundle {
                                        text: Text::from_section(
                                            "+",
                                            TextStyle {
                                                font: asset_server.load("fonts/FiraSans-Bold.ttf"),
                                                font_size: 16.0,
                                                color: Color::BLACK,
                                            },
                                        ),
                                        ..default()
                                    });
                                });
                        });
                });
        });
}
