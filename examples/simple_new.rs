use bevy::{
    prelude::*,
    render::render_graph_2::{StandardMaterial},
};

fn main() {
    AppBuilder::new().add_defaults().setup_world(setup).run();
}

fn setup(world: &mut World) {
    let cube = Mesh::load(MeshType::Cube);
    let plane = Mesh::load(MeshType::Plane { size: 10.0 });

    let (cube_handle, plane_handle) = {
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        (mesh_storage.add(cube), mesh_storage.add(plane))
    };

    world
        .build()
        // plane
        .add_archetype(NewMeshEntity {
            mesh: plane_handle.clone(),
            material: StandardMaterial {
                albedo: math::vec4(0.1, 0.2, 0.1, 1.0),
                everything_is_red: false,
            },
            ..Default::default()
        })
        // cube
        .add_archetype(NewMeshEntity {
            mesh: cube_handle.clone(),
            material: StandardMaterial {
                albedo: math::vec4(0.5, 0.4, 0.3, 1.0),
                everything_is_red: false,
            },
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        // light
        .add_archetype(LightEntity {
            light: Light {
                color: wgpu::Color {
                    r: 0.8,
                    g: 0.8,
                    b: 0.8,
                    a: 1.0,
                },
                fov: f32::to_radians(60.0),
                depth: 0.1..50.0,
                target_view: None,
            },
            local_to_world: LocalToWorld::identity(),
            translation: Translation::new(4.0, -4.0, 5.0),
            rotation: Rotation::from_euler_angles(0.0, 0.0, 0.0),
        })
        // camera
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
        .build();
}
