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

    commands
        .spawn(NodeBundle {
            style: Style {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::Center,
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexStart,
                        margin: UiRect {
                            bottom: Val::Px(10.),
                            ..default()
                        },
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
                            "Fully Opaque (100% black bg)",
                            TextStyle {
                                font_size: 30.0,
                                color: Color::WHITE,
                                ..Default::default()
                            },
                        ),
                        ..default()
                    });
                    parent.spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexStart,
                            margin: UiRect {
                                bottom: Val::Px(10.),
                                ..default()
                            },
                            row_gap: Val::Px(10.),
                            padding: UiRect::all(Val::Px(18.)),
                            ..default()
                        },
                        background_color: Color::GREEN.into(),
                        ..default()
                    });
                });
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexStart,
                        margin: UiRect {
                            bottom: Val::Px(10.),
                            ..default()
                        },
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
                            "Half Opaque (50%)",
                            TextStyle {
                                font_size: 30.0,
                                color: Color::WHITE,
                                ..Default::default()
                            },
                        ),
                        ..default()
                    });
                    parent.spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexStart,
                            margin: UiRect {
                                bottom: Val::Px(10.),
                                ..default()
                            },
                            row_gap: Val::Px(10.),
                            padding: UiRect::all(Val::Px(18.)),
                            ..default()
                        },
                        background_color: Color::GREEN.into(),
                        ..default()
                    });
                });
            parent
                .spawn(NodeBundle {
                    style: Style {
                        flex_direction: FlexDirection::Column,
                        justify_content: JustifyContent::FlexStart,
                        margin: UiRect {
                            bottom: Val::Px(10.),
                            ..default()
                        },
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
                            "Half Opaque (50%)",
                            TextStyle {
                                font_size: 30.0,
                                color: Color::WHITE,
                                ..Default::default()
                            },
                        ),
                        ..default()
                    });
                    parent.spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            justify_content: JustifyContent::FlexStart,
                            margin: UiRect {
                                bottom: Val::Px(10.),
                                ..default()
                            },
                            row_gap: Val::Px(10.),
                            padding: UiRect::all(Val::Px(18.)),
                            ..default()
                        },
                        background_color: Color::RED.into(),
                        ..default()
                    });
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::FlexStart,
                                margin: UiRect {
                                    bottom: Val::Px(10.),
                                    ..default()
                                },
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
                                    TextStyle {
                                        font_size: 30.0,
                                        color: Color::WHITE,
                                        ..Default::default()
                                    },
                                ),
                                ..default()
                            });
                            parent.spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::FlexStart,
                                    margin: UiRect {
                                        bottom: Val::Px(10.),
                                        ..default()
                                    },
                                    row_gap: Val::Px(10.),
                                    padding: UiRect::all(Val::Px(18.)),
                                    ..default()
                                },
                                background_color: Color::PURPLE.into(),
                                ..default()
                            });
                        });
                    parent
                        .spawn(NodeBundle {
                            style: Style {
                                flex_direction: FlexDirection::Column,
                                justify_content: JustifyContent::FlexStart,
                                margin: UiRect {
                                    bottom: Val::Px(10.),
                                    ..default()
                                },
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
                                    TextStyle {
                                        font_size: 30.0,
                                        color: Color::WHITE,
                                        ..Default::default()
                                    },
                                ),
                                ..default()
                            });
                            parent.spawn(NodeBundle {
                                style: Style {
                                    flex_direction: FlexDirection::Column,
                                    justify_content: JustifyContent::FlexStart,
                                    margin: UiRect {
                                        bottom: Val::Px(10.),
                                        ..default()
                                    },
                                    row_gap: Val::Px(10.),
                                    padding: UiRect::all(Val::Px(18.)),
                                    ..default()
                                },
                                background_color: Color::BLUE.into(),
                                ..default()
                            });
                        });
                });
        });
}
