use bevy::prelude::*;

fn main() {
    App::build().add_defaults().setup_world(setup).run();
}

fn setup(world: &mut World, resources: &mut Resources) {
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mut material_storage = resources
        .get_mut::<AssetStorage<StandardMaterial>>()
        .unwrap();
    let cube_handle = mesh_storage.add(Mesh::load(MeshType::Cube));
    let cube_material_handle = material_storage.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.3, 0.3),
        ..Default::default()
    });

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
                Color::rgb(0.1, 0.1, 0.1),
            ),
        })
        // top right anchor with vertical fill
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(0.0, 0.0),
                Anchors::new(1.0, 1.0, 0.0, 1.0),
                Margins::new(10.0, 100.0, 50.0, 100.0),
                Color::rgb(0.1, 0.1, 0.1),
            ),
        })
        // render order test: reddest in the back, whitest in the front
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(75.0, 75.0),
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
                Color::rgb(1.0, 0.1, 0.1),
            ),
        })
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(50.0, 50.0),
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
                Color::rgb(1.0, 0.3, 0.3),
            ),
        })
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(100.0, 100.0),
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
                Color::rgb(1.0, 0.5, 0.5),
            ),
        })
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(150.0, 150.0),
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
                Color::rgb(1.0, 0.7, 0.7),
            ),
        })
        // parenting
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(300.0, 300.0),
                Anchors::new(0.0, 0.0, 0.0, 0.0),
                Margins::new(0.0, 200.0, 0.0, 200.0),
                Color::rgb(0.1, 0.1, 1.0),
            ),
        })
        .add_children(|builder| {
            builder.add_entity(UiEntity {
                node: Node::new(
                    math::vec2(0.0, 0.0),
                    Anchors::new(0.0, 1.0, 0.0, 1.0),
                    Margins::new(20.0, 20.0, 20.0, 20.0),
                    Color::rgb(0.6, 0.6, 1.0),
                ),
            })
        })
        // alpha test
        .add_entity(UiEntity {
            node: Node::new(
                math::vec2(200.0, 200.0),
                Anchors::new(0.5, 0.5, 0.5, 0.5),
                Margins::new(0.0, 100.0, 0.0, 100.0),
                Color::rgba(1.0, 0.9, 0.9, 0.4),
            ),
        })
        .build();
}
