//! Example demonstrating gradients

use std::f32::consts::PI;

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2dBundle::default());
    let root = commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                flex_direction: FlexDirection::Column,
                width: Val::Percent(100.),
                height: Val::Percent(100.),
                row_gap: Val::Px(20.),
                column_gap: Val::Px(10.),
                ..Default::default()
            },
            ..Default::default()
        })
        .id();
    let dirs = spawn_dirs(&mut commands);
    commands.entity(root).add_child(dirs);
    let group = spawn_group(&mut commands);

    commands.entity(root).add_child(group);

    let group = spawn_group2(&mut commands);

    commands.entity(root).add_child(group);

    let group = spawn_group3(&mut commands);
    commands.entity(root).add_child(group);
}

fn spawn_dirs(commands: &mut Commands) -> Entity {
    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                flex_wrap: FlexWrap::Wrap,
                row_gap: Val::Px(10.),
                column_gap: Val::Px(10.),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|commands| {
            for angle in [-0.125 * PI, 0., 0.125 * PI] {
                commands
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        commands.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(100.),
                                height: Val::Px(100.),
                                ..Default::default()
                            },
                            background_color: LinearGradient::simple(
                                angle,
                                Color::WHITE,
                                Color::BLACK,
                            )
                            .into(),
                            ..Default::default()
                        });

                        commands.spawn(TextBundle::from_section(
                            angle.to_string(),
                            TextStyle::default(),
                        ));
                    });
            }
        })
        .id()
}

fn spawn_group(commands: &mut Commands) -> Entity {
    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                flex_wrap: FlexWrap::Wrap,
                row_gap: Val::Px(10.),
                column_gap: Val::Px(10.),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|commands| {
            for i in 0..4 {
                let angle = 0.5 * PI * i as f32;
                commands
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        commands.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(100.),
                                height: Val::Px(100.),
                                ..Default::default()
                            },
                            background_color: LinearGradient::simple(
                                angle,
                                Color::WHITE,
                                Color::BLACK,
                            )
                            .into(),
                            ..Default::default()
                        });

                        commands.spawn(TextBundle::from_section(
                            angle.to_string(),
                            TextStyle::default(),
                        ));
                    });
            }

            for i in 0..8 {
                let angle = 0.25 * PI * i as f32;
                commands
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        commands.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(100.),
                                height: Val::Px(100.),
                                ..Default::default()
                            },
                            background_color: LinearGradient::simple(
                                angle,
                                Color::WHITE,
                                Color::RED,
                            )
                            .into(),
                            ..Default::default()
                        });

                        commands.spawn(TextBundle::from_section(
                            angle.to_string(),
                            TextStyle::default(),
                        ));
                    });
            }
        })
        .id()
}

fn spawn_group2(commands: &mut Commands) -> Entity {
    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                flex_wrap: FlexWrap::Wrap,
                row_gap: Val::Px(10.),
                column_gap: Val::Px(10.),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|commands| {
            for i in 0..4 {
                let angle = -0.5 * PI * i as f32;
                commands
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        commands.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(100.),
                                height: Val::Px(100.),
                                ..Default::default()
                            },
                            background_color: LinearGradient::simple(
                                angle,
                                Color::WHITE,
                                Color::BLACK,
                            )
                            .into(),
                            ..Default::default()
                        });

                        commands.spawn(TextBundle::from_section(
                            angle.to_string(),
                            TextStyle::default(),
                        ));
                    });
            }

            for i in 0..8 {
                let angle = -0.25 * PI * i as f32;
                commands
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        commands.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(100.),
                                height: Val::Px(100.),
                                ..Default::default()
                            },
                            background_color: LinearGradient::simple(
                                angle,
                                Color::WHITE,
                                Color::GREEN,
                            )
                            .into(),
                            ..Default::default()
                        });

                        commands.spawn(TextBundle::from_section(
                            angle.to_string(),
                            TextStyle::default(),
                        ));
                    });
            }
        })
        .id()
}

fn spawn_group3(commands: &mut Commands) -> Entity {
    commands
        .spawn(NodeBundle {
            style: Style {
                align_items: AlignItems::Start,
                justify_content: JustifyContent::Start,
                flex_wrap: FlexWrap::Wrap,
                row_gap: Val::Px(10.),
                column_gap: Val::Px(10.),
                ..Default::default()
            },
            ..Default::default()
        })
        .with_children(|commands| {
            for i in 0..4 {
                let angle = -0.5 * PI * i as f32;
                commands
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        commands.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(100.),
                                height: Val::Px(100.),
                                ..Default::default()
                            },
                            background_color: LinearGradient::simple(
                                angle,
                                Color::WHITE,
                                Color::BLACK,
                            )
                            .into(),
                            ..Default::default()
                        });

                        commands.spawn(TextBundle::from_section(
                            angle.to_string(),
                            TextStyle::default(),
                        ));
                    });
            }

            for i in 0..8 {
                let angle = -0.25 * PI * i as f32;
                commands
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        commands.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(100.),
                                height: Val::Px(100.),
                                ..Default::default()
                            },
                            background_color: LinearGradient::new(
                                angle,
                                vec![
                                    ColorStop {
                                        color: Color::WHITE,
                                        point: Val::Percent(25.),
                                    },
                                    ColorStop {
                                        color: Color::RED,
                                        point: Val::Percent(75.),
                                    },
                                ],
                            )
                            .into(),
                            ..Default::default()
                        });

                        commands.spawn(TextBundle::from_section(
                            angle.to_string(),
                            TextStyle::default(),
                        ));
                    });
            }

            for i in 0..8 {
                let angle = -0.25 * PI * i as f32;
                commands
                    .spawn(NodeBundle {
                        style: Style {
                            flex_direction: FlexDirection::Column,
                            ..Default::default()
                        },
                        ..Default::default()
                    })
                    .with_children(|commands| {
                        commands.spawn(NodeBundle {
                            style: Style {
                                width: Val::Px(100.),
                                height: Val::Px(100.),
                                ..Default::default()
                            },
                            background_color: LinearGradient::new(
                                angle,
                                vec![
                                    ColorStop {
                                        color: Color::GREEN,
                                        point: Val::Auto,
                                    },
                                    ColorStop {
                                        color: Color::BLACK,
                                        point: Val::Percent(25.),
                                    },
                                    ColorStop {
                                        color: Color::WHITE,
                                        point: Val::Percent(25.),
                                    },
                                    ColorStop {
                                        color: Color::RED,
                                        point: Val::Percent(75.),
                                    },
                                    ColorStop {
                                        color: Color::BLACK,
                                        point: Val::Percent(75.),
                                    },
                                    ColorStop {
                                        color: Color::YELLOW,
                                        point: Val::Auto,
                                    },
                                ],
                            )
                            .into(),
                            ..Default::default()
                        });

                        commands.spawn(TextBundle::from_section(
                            angle.to_string(),
                            TextStyle::default(),
                        ));
                    });
            }
        })
        .id()
}
