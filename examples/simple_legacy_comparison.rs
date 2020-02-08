use bevy::prelude::*;
use rand::{rngs::StdRng, Rng, SeedableRng};
use std::collections::VecDeque;
fn main() {
    AppBuilder::new()
        .add_defaults_legacy()
        .add_system(build_move_system())
        .add_system(build_print_status_system())
        .setup_world(setup)
        .run();
}

fn build_move_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("Move")
        .read_resource::<Time>()
        .with_query(<(Write<Translation>, Write<Material>)>::query())
        .build(move |_, world, time, person_query| {
            for (mut translation, mut material) in person_query.iter_mut(world) {
                translation.0 += math::vec3(1.0, 0.0, 0.0) * time.delta_seconds;
            }
        })
}

fn build_print_status_system() -> Box<dyn Schedulable> {
    let mut elapsed = 0.0;
    let mut frame_time_total = 0.0;
    let mut frame_time_count = 0;
    let frame_time_max = 10;
    let mut frame_time_values = VecDeque::new();
    SystemBuilder::new("PrintStatus")
        .read_resource::<Time>()
        .build(move |_, _world, time, _queries| {
            elapsed += time.delta_seconds;
            frame_time_values.push_front(time.delta_seconds);
            frame_time_total += time.delta_seconds;
            frame_time_count += 1;
            if frame_time_count > frame_time_max {
                frame_time_count = frame_time_max;
                frame_time_total -= frame_time_values.pop_back().unwrap();
            }
            if elapsed > 1.0 {
                if frame_time_count > 0 && frame_time_total > 0.0 {
                    println!(
                        "fps: {}",
                        1.0 / (frame_time_total / frame_time_count as f32)
                    )
                }
                elapsed = 0.0;
            }
        })
}

fn setup(world: &mut World) {
    let cube = Mesh::load(MeshType::Cube);
    let plane = Mesh::load(MeshType::Plane { size: 10.0 });

    let (cube_handle, plane_handle) = {
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        (mesh_storage.add(cube), mesh_storage.add(plane))
    };

    let mut builder = world
        .build()
        // plane
        .add_archetype(MeshEntity {
            mesh: plane_handle.clone(),
            material: Material::new(Albedo::Color(math::vec4(0.1, 0.2, 0.1, 1.0))),
            local_to_world: LocalToWorld::identity(),
            translation: Translation::new(0.0, 0.0, 0.0),
        })
        // cube
        .add_archetype(MeshEntity {
            mesh: cube_handle.clone(),
            material: Material::new(Albedo::Color(math::vec4(0.5, 0.3, 0.3, 1.0))),
            local_to_world: LocalToWorld::identity(),
            translation: Translation::new(0.0, 0.0, 1.0),
        })
        // light
        .add_archetype(LightEntity {
            light: Light {
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
        });

    let mut rng = StdRng::from_entropy();
    for _ in 0..10000 {
        builder = builder.add_archetype(MeshEntity {
            mesh: cube_handle.clone(),
            material: Material::new(Albedo::Color(math::vec4(0.5, 0.3, 0.3, 1.0))),
            local_to_world: LocalToWorld::identity(),
            translation: Translation::new(
                rng.gen_range(-50.0, 50.0),
                rng.gen_range(-50.0, 50.0),
                0.0,
            ),
        });
    }

    builder.build()
}
