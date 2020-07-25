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
    // let texture_handle = asset_server
    //     .load_sync(&mut textures, "assets/branding/bevy_logo_dark_big.png")
    //     .unwrap();

    // let texture = textures.get(&texture_handle).unwrap();
    // let aspect = texture.aspect();

    commands
        // ui camera
        .spawn(UiCameraComponents::default())
        // root node
        .spawn(NodeComponents {
            flex: Flex {
                size: Size {
                    width: Dimension::Percent(1.0),
                    height: Dimension::Percent(1.0),
                },
                justify_content: JustifyContent::SpaceBetween,
                ..Default::default()
            },
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                // left vertical fill
                .spawn(NodeComponents {
                    flex: Flex {
                        size: Size {
                            width: Dimension::Points(200.0),
                            height: Dimension::Percent(1.0),
                        },
                        border: Rect {
                            start: Dimension::Points(10.0),
                            end: Dimension::Points(10.0),
                            top: Dimension::Points(10.0),
                            bottom: Dimension::Points(10.0),
                        },
                        ..Default::default()
                    },
                    material: materials.add(Color::rgb(0.02, 0.02, 0.8).into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent
                        .spawn(NodeComponents {
                            flex: Flex {
                                size: Size {
                                    width: Dimension::Percent(1.0),
                                    height: Dimension::Percent(1.0),
                                },
                                border: Rect {
                                    bottom: Dimension::Points(5.0),
                                    start: Dimension::Points(5.0),
                                    ..Default::default()
                                },
                                align_items: AlignItems::FlexEnd,
                                ..Default::default()
                            },
                            material: materials.add(Color::rgb(0.8, 0.02, 0.02).into()),
                            ..Default::default()
                        })
                        .with_children(|parent| {
                            parent.spawn(TextComponents {
                                flex: Flex {
                                    size: Size {
                                        width: Dimension::Points(100.0),
                                        height: Dimension::Points(30.0),
                                    },
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
                                        ..Default::default()
                                    },
                                },
                                ..Default::default()
                            });
                        });
                })
                // right vertical fill
                .spawn(NodeComponents {
                    flex: Flex {
                        size: Size {
                            width: Dimension::Points(100.0),
                            height: Dimension::Percent(1.0),
                        },
                        border: Rect {
                            start: Dimension::Points(10.0),
                            end: Dimension::Points(10.0),
                            top: Dimension::Points(10.0),
                            bottom: Dimension::Points(10.0),
                        },
                        ..Default::default()
                    },
                    material: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
                    ..Default::default()
                });

            // // left vertical fill

            // .spawn(NodeComponents {
            //     flex: Flex {
            //         size: Size {
            //             width: Dimension::Percent(0.20),
            //             height: Dimension::Percent(0.20),
            //         },
            //         justify_content: JustifyContent::FlexEnd,
            //         align_items: AlignItems::FlexEnd,
            //         ..Default::default()
            //     },
            //     material: materials.add(Color::rgb(0.02, 0.8, 0.02).into()),
            //     ..Default::default()
            // })
            // .with_children(|parent| {
            //     parent.spawn(NodeComponents {
            //         flex: Flex {
            //             size: Size {
            //                 width: Dimension::Percent(0.50),
            //                 height: Dimension::Percent(0.50),
            //             },
            //             justify_content: JustifyContent::FlexEnd,
            //             align_items: AlignItems::FlexEnd,
            //             ..Default::default()
            //         },
            //         material: materials.add(Color::rgb(0.8, 0.02, 0.02).into()),
            //         ..Default::default()
            //     });
            // });
            // // right vertical fill
            // .spawn(NodeComponents {
            //     node: Node::new(Anchors::RIGHT_FULL, Margins::new(10.0, 100.0, 100.0, 100.0)),
            //     material: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
            //     ..Default::default()
            // })
            // // render order test: reddest in the back, whitest in the front
            // .spawn(NodeComponents {
            //     node: Node::positioned(
            //         Vec2::new(75.0, 60.0),
            //         Anchors::CENTER,
            //         Margins::new(0.0, 100.0, 0.0, 100.0),
            //     ),
            //     material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
            //     ..Default::default()
            // })
            // .spawn(NodeComponents {
            //     node: Node::positioned(
            //         Vec2::new(50.0, 35.0),
            //         Anchors::CENTER,
            //         Margins::new(0.0, 100.0, 0.0, 100.0),
            //     ),
            //     material: materials.add(Color::rgb(1.0, 0.3, 0.3).into()),
            //     ..Default::default()
            // })
            // .spawn(NodeComponents {
            //     node: Node::positioned(
            //         Vec2::new(100.0, 85.0),
            //         Anchors::CENTER,
            //         Margins::new(0.0, 100.0, 0.0, 100.0),
            //     ),
            //     material: materials.add(Color::rgb(1.0, 0.5, 0.5).into()),
            //     ..Default::default()
            // })
            // .spawn(NodeComponents {
            //     node: Node::positioned(
            //         Vec2::new(150.0, 135.0),
            //         Anchors::CENTER,
            //         Margins::new(0.0, 100.0, 0.0, 100.0),
            //     ),
            //     material: materials.add(Color::rgb(1.0, 0.7, 0.7).into()),
            //     ..Default::default()
            // })
            // // parenting
            // .spawn(NodeComponents {
            //     node: Node::positioned(
            //         Vec2::new(210.0, 0.0),
            //         Anchors::BOTTOM_LEFT,
            //         Margins::new(0.0, 200.0, 10.0, 210.0),
            //     ),
            //     material: materials.add(Color::rgb(0.1, 0.1, 1.0).into()),
            //     ..Default::default()
            // })
            // .with_children(|parent| {
            //     parent.spawn(NodeComponents {
            //         node: Node::new(Anchors::FULL, Margins::new(20.0, 20.0, 20.0, 20.0)),
            //         material: materials.add(Color::rgb(0.6, 0.6, 1.0).into()),
            //         ..Default::default()
            //     });
            // })
            // // alpha test
            // .spawn(NodeComponents {
            //     node: Node::positioned(
            //         Vec2::new(200.0, 185.0),
            //         Anchors::CENTER,
            //         Margins::new(0.0, 100.0, 0.0, 100.0),
            //     ),
            //     material: materials.add(Color::rgba(1.0, 0.9, 0.9, 0.4).into()),
            //     draw: Draw {
            //         is_transparent: true,
            //         ..Default::default()
            //     },
            //     ..Default::default()
            // })
            // // texture
            // .spawn(NodeComponents {
            //     node: Node::new(
            //         Anchors::CENTER_TOP,
            //         Margins::new(-250.0, 250.0, 510.0 * aspect, 10.0),
            //     ),
            //     material: materials.add(ColorMaterial::texture(texture_handle)),
            //     draw: Draw {
            //         is_transparent: true,
            //         ..Default::default()
            //     },
            //     ..Default::default()
            // });
        })
        .spawn(NodeComponents {
            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
            ..Default::default()
        }) 
        ;
}
