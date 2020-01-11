use bevy::{
    asset::{Asset, AssetStorage},
    math::{Mat4, Vec3},
    render::*,
    *,
};

fn main() {
    AppBuilder::new().add_defaults().setup_world(setup).run();
}

fn setup(world: &mut World) {
    let cube = Mesh::load(MeshType::Cube);
    let plane = Mesh::load(MeshType::Plane { size: 10.0 });

    let (cube_handle, plane_handle) = {
        let mut mesh_storage = world
            .resources
            .get_mut::<AssetStorage<Mesh, MeshType>>()
            .unwrap();
        (
            mesh_storage.add(cube, "cube"),
            mesh_storage.add(plane, "plane"),
        )
    };

    // plane
    world.insert(
        (),
        vec![(
            plane_handle.clone(),
            Material::new(math::vec4(0.1, 0.2, 0.1, 1.0)),
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, 0.0),
        )],
    );

    // cube
    world.insert(
        (),
        vec![(
            cube_handle,
            Material::new(math::vec4(0.5, 0.3, 0.3, 1.0)),
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, 1.0),
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
            Material::new(math::vec4(0.5, 0.3, 0.3, 1.0)),
            LocalToWorld::identity(),
            Translation::new(4.0, -4.0, 5.0),
            Rotation::from_euler_angles(0.0, 0.0, 0.0),
        )],
    );

    // camera
    world.insert(
        (),
        vec![(
            Camera::new(CameraType::Projection {
                fov: std::f32::consts::PI / 4.0,
                near: 1.0,
                far: 1000.0,
                aspect_ratio: 1.0,
            }),
            ActiveCamera,
            LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, -8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
        )],
    );
}
