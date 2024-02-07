//! Demonstrates how opacity with a hierarchy works

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .insert_resource(ClearColor(Color::WHITE))
        .run();
}

fn setup(mut commands: Commands) {
    // Camera
    commands.spawn(Camera2dBundle::default());

    let text_style = TextStyle {
        font_size: 30.0,
        color: Color::WHITE,
        ..Default::default()
    };
    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(18.)),
                row_gap: Val::Px(10.),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            // 100% Opacity section
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.),
                        padding: UiRect::all(Val::Px(18.)),
                        ..default()
                    },
                    background_color: Color::BLACK.into(),
                    opacity: Opacity(1.0),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle {
                        text: Text::from_section("Fully Opaque (100% black bg)", text_style.clone()),
                        ..default()
                    });
                    parent.spawn(NodeBundle {
                        style: Style {
                            height: Val::Px(36.),
                            ..default()
                        },
                        background_color: Color::GREEN.into(),
                        ..default()
                    });
                });

            // 50% Opacity section
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.),
                        padding: UiRect::all(Val::Px(18.)),
                        ..default()
                    },
                    background_color: Color::BLACK.into(),
                    opacity: Opacity(0.5),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle {
                        text: Text::from_section("Half Opaque (50%)", text_style.clone()),
                        ..default()
                    });
                    parent.spawn(NodeBundle {
                        style: Style {
                            height: Val::Px(36.),
                            ..default()
                        },
                        background_color: Color::GREEN.into(),
                        ..default()
                    });
                });

            // Nested 50% Opacity section
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(10.),
                        padding: UiRect::all(Val::Px(18.)),
                        ..default()
                    },
                    background_color: Color::BLACK.into(),
                    opacity: Opacity(0.5),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn(TextBundle {
                        text: Text::from_section("Half Opaque (50%)", text_style.clone()),
                        ..default()
                    });
                    parent.spawn(NodeBundle {
                        style: Style {
                            height: Val::Px(36.),
                            ..default()
                        },
                        background_color: Color::RED.into(),
                        ..default()
                    });
                    // Child 1
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(10.),
                                padding: UiRect::all(Val::Px(18.)),
                                ..default()
                            },
                            background_color: Color::BLACK.into(),
                            opacity: Opacity(1.0),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn(TextBundle {
                                text: Text::from_section(
                                    "Half Opaque (75% black bg over 2 layers)",
                                    text_style.clone(),
                                ),
                                ..default()
                            });
                            parent.spawn(NodeBundle {
                                style: Style {
                                    height: Val::Px(36.),
                                    ..default()
                                },
                                background_color: Color::PURPLE.into(),
                                ..default()
                            });
                        });
                    // Child 2
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                row_gap: Val::Px(10.),
                                padding: UiRect::all(Val::Px(18.)),
                                ..default()
                            },
                            background_color: Color::BLACK.into(),
                            opacity: Opacity(0.5),
                            ..default()
                        })
                        .with_children(|parent| {
                            parent.spawn(TextBundle {
                                text: Text::from_section(
                                    "Quarter Opaque (62.5% over 2 layers)",
                                    text_style.clone(),
                                ),
                                ..default()
                            });
                            parent.spawn(NodeBundle {
                                style: Style {
                                    height: Val::Px(36.),
                                    ..default()
                                },
                                background_color: Color::BLUE.into(),
                                ..default()
                            });
                        });
                });
        });
}
