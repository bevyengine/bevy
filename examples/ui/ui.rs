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

    let font_handle = asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap();
    let texture = textures.get(&texture_handle).unwrap();
    let aspect = texture.aspect();

    commands
        // ui camera
        .spawn(OrthographicCameraComponents::default())
        // root node
        .spawn(NodeComponents {
            node: Node::new(Anchors::FULL, Margins::default()),
            material: materials.add(Color::NONE.into()),
            ..Default::default()
        })
        .with_children(|parent| {
            parent
                // left vertical fill
                .spawn(NodeComponents {
                    node: Node::new(Anchors::LEFT_FULL, Margins::new(10.0, 200.0, 10.0, 10.0)),
                    material: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn(LabelComponents {
                        node: Node::new(Anchors::TOP_LEFT, Margins::new(10.0, 200.0, 40.0, 10.0)),
                        label: Label {
                            text: "Text Label".to_string(),
                            font: font_handle,
                            style: TextStyle {
                                font_size: 30.0,
                                color: Color::WHITE,
                                ..Default::default()
                            },
                        },
                        ..Default::default()
                    });
                })
                // right vertical fill
                .spawn(NodeComponents {
                    node: Node::new(Anchors::RIGHT_FULL, Margins::new(10.0, 100.0, 100.0, 100.0)),
                    material: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
                    ..Default::default()
                })
                // render order test: reddest in the back, whitest in the front
                .spawn(NodeComponents {
                    node: Node::positioned(
                        Vec2::new(75.0, 60.0),
                        Anchors::CENTER,
                        Margins::new(0.0, 100.0, 0.0, 100.0),
                    ),
                    material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
                    ..Default::default()
                })
                .spawn(NodeComponents {
                    node: Node::positioned(
                        Vec2::new(50.0, 35.0),
                        Anchors::CENTER,
                        Margins::new(0.0, 100.0, 0.0, 100.0),
                    ),
                    material: materials.add(Color::rgb(1.0, 0.3, 0.3).into()),
                    ..Default::default()
                })
                .spawn(NodeComponents {
                    node: Node::positioned(
                        Vec2::new(100.0, 85.0),
                        Anchors::CENTER,
                        Margins::new(0.0, 100.0, 0.0, 100.0),
                    ),
                    material: materials.add(Color::rgb(1.0, 0.5, 0.5).into()),
                    ..Default::default()
                })
                .spawn(NodeComponents {
                    node: Node::positioned(
                        Vec2::new(150.0, 135.0),
                        Anchors::CENTER,
                        Margins::new(0.0, 100.0, 0.0, 100.0),
                    ),
                    material: materials.add(Color::rgb(1.0, 0.7, 0.7).into()),
                    ..Default::default()
                })
                // parenting
                .spawn(NodeComponents {
                    node: Node::positioned(
                        Vec2::new(210.0, 0.0),
                        Anchors::BOTTOM_LEFT,
                        Margins::new(0.0, 200.0, 10.0, 210.0),
                    ),
                    material: materials.add(Color::rgb(0.1, 0.1, 1.0).into()),
                    ..Default::default()
                })
                .with_children(|parent| {
                    parent.spawn(NodeComponents {
                        node: Node::new(Anchors::FULL, Margins::new(20.0, 20.0, 20.0, 20.0)),
                        material: materials.add(Color::rgb(0.6, 0.6, 1.0).into()),
                        ..Default::default()
                    });
                })
                // alpha test
                .spawn(NodeComponents {
                    node: Node::positioned(
                        Vec2::new(200.0, 185.0),
                        Anchors::CENTER,
                        Margins::new(0.0, 100.0, 0.0, 100.0),
                    ),
                    material: materials.add(Color::rgba(1.0, 0.9, 0.9, 0.4).into()),
                    ..Default::default()
                })
                // texture
                .spawn(NodeComponents {
                    node: Node::new(
                        Anchors::CENTER_TOP,
                        Margins::new(-250.0, 250.0, 510.0 * aspect, 10.0),
                    ),
                    material: materials.add(ColorMaterial::texture(texture_handle)),
                    ..Default::default()
                });
        });
}
