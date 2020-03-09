use bevy::prelude::*;

struct Rotator;

fn main() {
    AppBuilder::new()
        .add_defaults()
        .setup_world(setup)
        .add_system(build_rotator_system())
        .run();
}

// rotates the parent, which will result in the child also rotating
fn build_rotator_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("Rotator")
        .read_resource::<Time>()
        .with_query(<(Write<Rotator>, Write<Rotation>)>::query())
        .build(move |_, world, time, rotator_query| {
            for (_rotator, mut rotation) in rotator_query.iter_mut(world) {
                rotation.0 = rotation.0 * Quat::from_rotation_x(3.0 * time.delta_seconds);
            }
        })
}

fn setup(world: &mut World, resources: &mut Resources) {
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let cube_handle = mesh_storage.add(Mesh::load(MeshType::Cube));

    world
        .build()
        // parent cube
        .add_entity(MeshEntity {
            mesh: cube_handle,
            material: StandardMaterial {
                albedo: math::vec4(0.5, 0.4, 0.3, 1.0).into(),
            },
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        .add(Rotator)
        .add_children(|builder| {
            // cube
            builder.add_entity(MeshEntity {
                mesh: cube_handle,
                material: StandardMaterial {
                    albedo: math::vec4(0.5, 0.4, 0.3, 1.0).into(),
                },
                translation: Translation::new(0.0, 0.0, 3.0),
                ..Default::default()
            })
        })
        // light
        .add_entity(LightEntity {
            translation: Translation::new(4.0, -4.0, 5.0),
            rotation: Rotation::from_euler_angles(0.0, 0.0, 0.0),
            ..Default::default()
        })
        // camera
        .add_entity(CameraEntity {
            camera: Camera::new(CameraType::Projection {
                fov: std::f32::consts::PI / 4.0,
                near: 1.0,
                far: 1000.0,
                aspect_ratio: 1.0,
            }),
            active_camera: ActiveCamera,
            local_to_world: LocalToWorld(Mat4::look_at_rh(
                Vec3::new(5.0, 10.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
        })
        .build();
}
