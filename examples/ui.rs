use bevy::prelude::*;

fn main() {
    AppBuilder::new().add_defaults().setup_world(setup).run();
}

fn setup(world: &mut World) {
    let cube = Mesh::load(MeshType::Cube);
    let cube_handle = {
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        mesh_storage.add(cube)
    };

    world
        .build()
        // cube
        .add_archetype(MeshEntity {
            mesh: cube_handle.clone(),
            material: StandardMaterial {
                albedo: math::vec4(0.5, 0.3, 0.3, 1.0),
                everything_is_red: false,
            },
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        // light
        .add_archetype(LightEntity {
            translation: Translation::new(4.0, -4.0, 5.0),
            ..Default::default()
        })
        // 3d camera
        .add_archetype(CameraEntity {
            camera: Camera::new(CameraType::Projection {
                fov: std::f32::consts::PI / 4.0,
                near: 1.0,
                far: 1000.0,
                aspect_ratio: 1.0,
            }),
            active_camera: ActiveCamera,
            local_to_world: LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
        })
        // 2d camera
        .add_archetype(Camera2dEntity {
            camera: Camera::new(CameraType::Orthographic {
                left: 0.0,
                right: 0.0,
                bottom: 0.0,
                top: 0.0,
                near: 0.0,
                far: 1.0,
            }),
            active_camera_2d: ActiveCamera2d,
        })
        .build();

    // bottom left anchor with vertical fill
    world.insert(
        (),
        vec![(Node::new(
            math::vec2(0.0, 0.0),
            Anchors::new(0.0, 0.0, 0.0, 1.0),
            Margins::new(10.0, 200.0, 10.0, 10.0),
            math::vec4(0.1, 0.1, 0.1, 1.0),
        ),)],
    );

    // top right anchor with vertical fill
    world.insert(
        (),
        vec![(Node::new(
            math::vec2(0.0, 0.0),
            Anchors::new(1.0, 1.0, 0.0, 1.0),
            Margins::new(10.0, 100.0, 50.0, 100.0),
            math::vec4(0.1, 0.1, 0.1, 1.0),
        ),)],
    );

    // render order test: reddest in the back, whitest in the front
    world.insert(
        (),
        vec![(Node::new(
            math::vec2(75.0, 75.0),
            Anchors::new(0.5, 0.5, 0.5, 0.5),
            Margins::new(0.0, 100.0, 0.0, 100.0),
            math::vec4(1.0, 0.1, 0.1, 1.0),
        ),)],
    );

    world.insert(
        (),
        vec![(Node::new(
            math::vec2(50.0, 50.0),
            Anchors::new(0.5, 0.5, 0.5, 0.5),
            Margins::new(0.0, 100.0, 0.0, 100.0),
            math::vec4(1.0, 0.3, 0.3, 1.0),
        ),)],
    );

    world.insert(
        (),
        vec![(Node::new(
            math::vec2(100.0, 100.0),
            Anchors::new(0.5, 0.5, 0.5, 0.5),
            Margins::new(0.0, 100.0, 0.0, 100.0),
            math::vec4(1.0, 0.5, 0.5, 1.0),
        ),)],
    );

    world.insert(
        (),
        vec![(Node::new(
            math::vec2(150.0, 150.0),
            Anchors::new(0.5, 0.5, 0.5, 0.5),
            Margins::new(0.0, 100.0, 0.0, 100.0),
            math::vec4(1.0, 0.7, 0.7, 1.0),
        ),)],
    );

    // parenting
    let parent = *world
        .insert(
            (),
            vec![(Node::new(
                math::vec2(300.0, 300.0),
                Anchors::new(0.0, 0.0, 0.0, 0.0),
                Margins::new(0.0, 200.0, 0.0, 200.0),
                math::vec4(0.1, 0.1, 1.0, 1.0),
            ),)],
        )
        .first()
        .unwrap();

    world.insert(
        (),
        vec![(
            Node::new(
                math::vec2(0.0, 0.0),
                Anchors::new(0.0, 1.0, 0.0, 1.0),
                Margins::new(20.0, 20.0, 20.0, 20.0),
                math::vec4(0.6, 0.6, 1.0, 1.0),
            ),
            Parent(parent),
        )],
    );

    // alpha test
    world.insert(
        (),
        vec![(Node::new(
            math::vec2(200.0, 200.0),
            Anchors::new(0.5, 0.5, 0.5, 0.5),
            Margins::new(0.0, 100.0, 0.0, 100.0),
            math::vec4(1.0, 0.9, 0.9, 0.4),
        ),)],
    );
}
