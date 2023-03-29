//! This example demonstrates rotated UI elements.

use bevy::{prelude::*, winit::WinitSettings};

#[derive(Resource)]
struct Center(Entity);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        // Only run the app when there is user input. This will significantly reduce CPU/GPU use.
        .insert_resource(WinitSettings::desktop_app())
        .add_systems(Startup, setup)
        .add_systems(Update, button_system)
        .run();
}

const NORMAL_BUTTON: Color = Color::WHITE;
const HOVERED_BUTTON: Color = Color::YELLOW;
const PRESSED_BUTTON: Color = Color::RED;

#[derive(Component)]
pub struct RotateButton(pub f32);

#[derive(Component)]
pub struct RotatingPanel;

fn button_system(
    mut interaction_query: Query<
        (&Interaction, &mut BackgroundColor, Option<&RotateButton>),
        (Changed<Interaction>, With<Button>),
    >,
    mut rotator_query: Query<&mut NodeRotation, With<RotatingPanel>>,
) {
    for (interaction, mut color, maybe_rotate) in &mut interaction_query {
        match *interaction {
            Interaction::Clicked => {
                *color = PRESSED_BUTTON.into();
                if let Some(maybe_rotate) = maybe_rotate {
                    for mut rotator in rotator_query.iter_mut() {
                        rotator.0 += maybe_rotate.0;
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
                                margin: UiRect::all(Val::Px(100.)),
                                ..default()
                            },
                            background_color: Color::WHITE.into(),
                            ..default()
                        })
                        .insert(RotateButton(-std::f32::consts::PI / 8.))
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
                        .insert(RotatingPanel)
                        .insert(NodeRotation::default())
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
                        .spawn(ButtonBundle {
                            style: Style {
                                size: Size::all(Val::Px(50.)),
                                align_items: AlignItems::Center,
                                justify_content: JustifyContent::Center,
                                margin: UiRect::all(Val::Px(100.)),
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
                });
        });
}
