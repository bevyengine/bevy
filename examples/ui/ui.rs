use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    let texture_handle = asset_server
        .load_sync(&mut textures, "assets/branding/bevy_logo_dark_big.png")
        .unwrap();

    let texture = textures.get(&texture_handle).unwrap();
    let aspect = texture.aspect();

    commands
        // ui camera
        .spawn(UiCameraComponents::default())
        // root node
        .spawn(NodeComponents {
            style: Style {
                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                justify_content: JustifyContent::SpaceBetween,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                // left vertical fill (border)
                .spawn(NodeComponents {
                    style: Style {
                        size: Size::new(Val::Px(200.0), Val::Percent(100.0)),
                        border: Rect::all(Val::Px(2.0)),
                        ..Default::default()
                    },
                    material: materials.add(Color::rgb(0.4, 0.4, 0.4).into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent
                        // left vertical fill (content)
                        .spawn(NodeComponents {
                            style: Style {
                                size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                                align_items: AlignItems::FlexEnd,
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            // text
                            parent.spawn(TextComponents {
                                style: Style {
                                    margin: Rect::all(Val::Px(5.0)),
                                    ..Default::default()
                                },
                                text: Text {
                                    value: "Text Example".to_string(),
                                    font: asset_server
                                        .load("assets/fonts/FiraSans-Bold.ttf")
                                        .unwrap(),
                                    style: TextStyle {
                                        font_size: 30.0,
                                        color: Color::WHITE,
                                    },
                                },
                                ..Default::default()
                            });
                        });
                })
                // right vertical fill
                .spawn(NodeComponents {
                    style: Style {
                        size: Size::new(Val::Px(200.0), Val::Percent(100.0)),
                        ..Default::default()
                    },
                    material: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
                    ..Default::default()
                })
                // Absolute positioning
                .spawn(NodeComponents {
                    style: Style {
                        size: Size::new(Val::Px(200.0), Val::Px(200.0)),
                        position_type: PositionType::Absolute,
                        position: Rect {
                            left: Val::Px(210.0),
                            bottom: Val::Px(10.0),
                            ..Default::default()
                        },
                        border: Rect::all(Val::Px(20.0)),
                        ..Default::default()
                    },
                    material: materials.add(Color::rgb(0.1, 0.1, 1.0).into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn(NodeComponents {
                        style: Style {
                            size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                            ..Default::default()
                        },
                        material: materials.add(Color::rgb(0.6, 0.6, 1.0).into()),
                        ..Default::default()
                    });
                })
                // render order test: reddest in the back, whitest in the front (flex center)
                .spawn(NodeComponents {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        position_type: PositionType::Absolute,
                        align_items: AlignItems::Center,
                        justify_content: JustifyContent::Center,
                        ..Default::default()
                    },
                    material: materials.add(Color::NONE.into()),
                    draw: Draw {
                        is_transparent: true,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent
                        .spawn(NodeComponents {
                            style: Style {
                                size: Size::new(Val::Px(100.0), Val::Px(100.0)),
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent
                                .spawn(NodeComponents {
                                    style: Style {
                                        size: Size::new(Val::Px(100.0), Val::Px(100.0)),
                                        position_type: PositionType::Absolute,
                                        position: Rect {
                                            left: Val::Px(20.0),
                                            bottom: Val::Px(20.0),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    material: materials.add(Color::rgb(1.0, 0.3, 0.3).into()),
                                    ..Default::default()
                                })
                                .spawn(NodeComponents {
                                    style: Style {
                                        size: Size::new(Val::Px(100.0), Val::Px(100.0)),
                                        position_type: PositionType::Absolute,
                                        position: Rect {
                                            left: Val::Px(40.0),
                                            bottom: Val::Px(40.0),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    material: materials.add(Color::rgb(1.0, 0.5, 0.5).into()),
                                    ..Default::default()
                                })
                                .spawn(NodeComponents {
                                    style: Style {
                                        size: Size::new(Val::Px(100.0), Val::Px(100.0)),
                                        position_type: PositionType::Absolute,
                                        position: Rect {
                                            left: Val::Px(60.0),
                                            bottom: Val::Px(60.0),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    material: materials.add(Color::rgb(1.0, 0.7, 0.7).into()),
                                    ..Default::default()
                                })
                                // alpha test
                                .spawn(NodeComponents {
                                    style: Style {
                                        size: Size::new(Val::Px(100.0), Val::Px(100.0)),
                                        position_type: PositionType::Absolute,
                                        position: Rect {
                                            left: Val::Px(80.0),
                                            bottom: Val::Px(80.0),
                                            ..Default::default()
                                        },
                                        ..Default::default()
                                    },
                                    material: materials.add(Color::rgba(1.0, 0.9, 0.9, 0.4).into()),
                                    draw: Draw {
                                        is_transparent: true,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                });
                        });
                })
                // bevy logo (flex center)
                .spawn(NodeComponents {
                    style: Style {
                        size: Size::new(Val::Percent(100.0), Val::Percent(100.0)),
                        position_type: PositionType::Absolute,
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::FlexEnd,
                        ..Default::default()
                    },
                    material: materials.add(Color::NONE.into()),
                    draw: Draw {
                        is_transparent: true,
                        ..Default::default()
                    },
                    ..Default::default()
                })
                .with_children(|parent| {
                    // bevy logo (image)
                    parent.spawn(NodeComponents {
                        style: Style {
                            min_size: Size::new(Val::Px(500.0), Val::Px(500.0 * aspect)),
                            ..Default::default()
                        },
                        material: materials.add(ColorMaterial::texture(texture_handle)),
                        draw: Draw {
                            is_transparent: true,
                            ..Default::default()
                        },
                        ..Default::default()
                    });
                });
        });
}
