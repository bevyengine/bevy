use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

fn setup(
    command_buffer: &mut CommandBuffer,
    asset_server: Res<AssetServer>,
    mut textures: ResMut<Assets<Texture>>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    // TODO: "background" 3D temporarily disabled until depth mismatch is fixed
    // let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    // let mut material_storage = resources
    //     .get_mut::<AssetStorage<StandardMaterial>>()
    //     .unwrap();
    // let cube_handle = mesh_storage.add(Mesh::from(shape::Cube));
    // let cube_material_handle = material_storage.add(StandardMaterial {
    //     albedo: Color::rgb(0.5, 0.4, 0.3),
    //     ..Default::default()
    // });

    let texture_handle = asset_server
        .load_sync(&mut textures, "assets/branding/bevy_logo_dark_big.png")
        .unwrap();

    let font_handle = asset_server.load("assets/fonts/FiraSans-Bold.ttf").unwrap();
    let texture = textures.get(&texture_handle).unwrap();
    let aspect = texture.aspect();

    command_buffer
        .build()
        // // cube
        // .add_entity(MeshEntity {
        //     mesh: cube_handle,
        //     material: cube_material_handle,
        //     translation: Translation::new(0.0, 0.0, 1.0),
        //     ..Default::default()
        // })
        // // light
        // .add_entity(LightEntity {
        //     translation: Translation::new(4.0, -4.0, 5.0),
        //     ..Default::default()
        // })
        // // 3d camera
        // .add_entity(CameraEntity {
        //     transform: Transform(Mat4::look_at_rh(
        //         Vec3::new(3.0, 8.0, 5.0),
        //         Vec3::new(0.0, 0.0, 0.0),
        //         Vec3::new(0.0, 0.0, 1.0),
        //     )),
        //     ..Default::default()
        // })
        // ui camera
        .add_entity(OrthographicCameraEntity::ui())
        // left vertical fill
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(0.0, 0.0),
                Anchors::LEFT_FULL,
                Margins::new(10.0, 200.0, 10.0, 10.0),
            ),
            material: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
            ..Default::default()
        })
        .add_children(|builder| {
            builder.add_entity(LabelEntity {
                node: Node::new(
                    math::vec2(0.0, 0.0),
                    Anchors::TOP_LEFT,
                    Margins::new(10.0, 200.0, 40.0, 10.0),
                ),
                label: Label {
                    text: "Text Label".to_string(),
                    font: font_handle,
                    style: TextStyle {
                        font_size: 30.0,
                        color: Color::WHITE,
                    },
                },
                ..Default::default()
            })
        })
        // right vertical fill
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(0.0, 0.0),
                Anchors::RIGHT_FULL,
                Margins::new(10.0, 100.0, 100.0, 100.0),
            ),
            material: materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
            ..Default::default()
        })
        // render order test: reddest in the back, whitest in the front
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(75.0, 60.0),
                Anchors::CENTER,
                Margins::new(0.0, 100.0, 0.0, 100.0),
            ),
            material: materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
            ..Default::default()
        })
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(50.0, 35.0),
                Anchors::CENTER,
                Margins::new(0.0, 100.0, 0.0, 100.0),
            ),
            material: materials.add(Color::rgb(1.0, 0.3, 0.3).into()),
            ..Default::default()
        })
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(100.0, 85.0),
                Anchors::CENTER,
                Margins::new(0.0, 100.0, 0.0, 100.0),
            ),
            material: materials.add(Color::rgb(1.0, 0.5, 0.5).into()),
            ..Default::default()
        })
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(150.0, 135.0),
                Anchors::CENTER,
                Margins::new(0.0, 100.0, 0.0, 100.0),
            ),
            material: materials.add(Color::rgb(1.0, 0.7, 0.7).into()),
            ..Default::default()
        })
        // parenting
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(210.0, 0.0),
                Anchors::BOTTOM_LEFT,
                Margins::new(0.0, 200.0, 10.0, 210.0),
            ),
            material: materials.add(Color::rgb(0.1, 0.1, 1.0).into()),
            ..Default::default()
        })
        .add_children(|builder| {
            builder.add_entity(UiEntity {
                node: Node::new(
                    math::vec2(0.0, 0.0),
                    Anchors::FULL,
                    Margins::new(20.0, 20.0, 20.0, 20.0),
                ),
                material: materials.add(Color::rgb(0.6, 0.6, 1.0).into()),
                ..Default::default()
            })
        })
        // alpha test
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(200.0, 185.0),
                Anchors::CENTER,
                Margins::new(0.0, 100.0, 0.0, 100.0),
            ),
            material: materials.add(Color::rgba(1.0, 0.9, 0.9, 0.4).into()),
            ..Default::default()
        })
        // texture
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(0.0, 0.0),
                Anchors::CENTER_TOP,
                Margins::new(-250.0, 250.0, 510.0 * aspect, 10.0),
            ),
            material: materials.add(ColorMaterial::texture(texture_handle)),
            ..Default::default()
        });
}
