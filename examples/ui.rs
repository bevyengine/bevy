use bevy::{
    asset::{Asset, AssetStorage},
    math::{Mat4, Vec3},
    render::*,
    ui::*,
    *,
};

fn main() {
    AppBuilder::new().add_defaults().setup_world(setup).run();
}

fn setup(world: &mut World) {
    let cube = Mesh::load(MeshType::Cube);
    let cube_handle = {
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        mesh_storage.add(cube)
    };

    // cube
    world.insert(
        (),
        vec![(
            cube_handle,
            LocalToWorld::identity(),
            Material::new(math::vec4(0.5, 0.3, 0.3, 1.0)),
        )],
    );

    // light
    world.insert(
        (),
        vec![(
            Light {
                color: wgpu::Color {
                    r: 0.8,
                    g: 0.8,
                    b: 0.5,
                    a: 1.0,
                },
                fov: f32::to_radians(60.0),
                depth: 0.1..50.0,
                target_view: None,
            },
            LocalToWorld::identity(),
            Translation::new(4.0, -4.0, 5.0),
            Rotation::from_euler_angles(0.0, 0.0, 0.0),
        )],
    );

    // 3d camera
    world.insert(
        (),
        vec![
            // camera
            (
                Camera::new(CameraType::Projection {
                    fov: std::f32::consts::PI / 4.0,
                    near: 1.0,
                    far: 1000.0,
                    aspect_ratio: 1.0,
                }),
                ActiveCamera,
                LocalToWorld(Mat4::look_at_rh(
                    Vec3::new(3.0, -5.0, 3.0),
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(0.0, 0.0, 1.0),
                )),
            ),
        ],
    );

    // 2d camera
    world.insert(
        (),
        vec![(
            Camera::new(CameraType::Orthographic {
                left: 0.0,
                right: 0.0,
                bottom: 0.0,
                top: 0.0,
                near: 0.0,
                far: 1.0,
            }),
            ActiveCamera2d,
        )],
    );

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
}
