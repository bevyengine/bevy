use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup)
        .run();
}

fn setup(world: &mut World, resources: &mut Resources) {
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mut material_storage = resources
        .get_mut::<AssetStorage<StandardMaterial>>()
        .unwrap();
    let cube_handle = mesh_storage.add(Mesh::from(shape::Cube));
    let cube_material_handle = material_storage.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.3, 0.3),
        ..Default::default()
    });

    let mut texture_storage = resources.get_mut::<AssetStorage<Texture>>().unwrap();
    let texture_path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/assets/branding/bevy_logo_dark_big.png"
    );
    let texture = Texture::load(TextureType::Png(texture_path.to_string()));
    let aspect = texture.aspect();
    let texture_handle = texture_storage.add(texture);

    let mut color_materials = resources.get_mut::<AssetStorage<ColorMaterial>>().unwrap();
    let blue_material_handle = color_materials.add(Color::rgb(0.6, 0.6, 1.0).into());

    world
        .build()
        // cube
        .add_entity(MeshEntity {
            mesh: cube_handle,
            material: cube_material_handle,
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        // light
        .add_entity(LightEntity {
            translation: Translation::new(4.0, -4.0, 5.0),
            ..Default::default()
        })
        // 3d camera
        .add_entity(CameraEntity {
            local_to_world: LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        })
        // 2d camera
        .add_entity(Camera2dEntity {
            ..Default::default()
        })
        // bottom left anchor with vertical fill
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(0.0, 0.0),
                Anchors::new(0.0, 0.0, 0.0, 1.0),
                Margins::new(10.0, 200.0, 10.0, 10.0),
            ),
            material: color_materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
            ..Default::default()
        })
        // top right anchor with vertical fill
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(0.0, 0.0),
                Anchors::new(1.0, 1.0, 0.0, 1.0),
                Margins::new(10.0, 100.0, 50.0, 100.0),
            ),
            material: color_materials.add(Color::rgb(0.02, 0.02, 0.02).into()),
            ..Default::default()
        })
        // render order test: reddest in the back, whitest in the front
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(75.0, 75.0),
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
            ),
            material: color_materials.add(Color::rgb(1.0, 0.0, 0.0).into()),
            ..Default::default()
        })
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(50.0, 50.0),
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
            ),
            material: color_materials.add(Color::rgb(1.0, 0.3, 0.3).into()),
            ..Default::default()
        })
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(100.0, 100.0),
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
            ),
            material: color_materials.add(Color::rgb(1.0, 0.5, 0.5).into()),
            ..Default::default()
        })
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(150.0, 150.0),
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
            ),
            material: color_materials.add(Color::rgb(1.0, 0.7, 0.7).into()),
            ..Default::default()
        })
        // parenting
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(300.0, 300.0),
                Anchors::new(0.0, 0.0, 0.0, 0.0),
                Margins::new(0.0, 200.0, 0.0, 200.0),
            ),
            material: color_materials.add(Color::rgb(0.1, 0.1, 1.0).into()),
            ..Default::default()
        })
        .add_children(|builder| {
            builder.add_entity(UiEntity {
                node: Node::new(
                    math::vec2(0.0, 0.0),
                    Anchors::new(0.0, 1.0, 0.0, 1.0),
                    Margins::new(20.0, 20.0, 20.0, 20.0),
                ),
                material: blue_material_handle,
                ..Default::default()
            });
        })
        // alpha test
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(200.0, 200.0),
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
            ),
            material: color_materials.add(Color::rgba(1.0, 0.9, 0.9, 0.4).into()),
            ..Default::default()
        })
        // texture
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(400.0, 100.0),
                Anchors::new(0.0, 0.0, 0.0, 0.0),
                Margins::new(0.0, 500.0, 0.0, 500.0 * aspect),
            ),
            material: color_materials.add(ColorMaterial::texture(texture_handle)),
            ..Default::default()
        });
}
