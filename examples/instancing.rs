use bevy::prelude::*;
use rand::{random, rngs::StdRng, Rng, SeedableRng};
use std::collections::VecDeque;

struct Person;

struct Velocity {
    pub value: math::Vec3,
}

struct NavigationPoint {
    pub target: math::Vec3,
}

struct Wander {
    pub duration_bounds: math::Vec2,
    pub distance_bounds: math::Vec2,
    pub duration: f32,
    pub elapsed: f32,
}

fn main() {
    AppBuilder::new()
        .setup_world(setup)
        .add_system(build_wander_system())
        .add_system(build_navigate_system())
        .add_system(build_move_system())
        .add_system(build_print_status_system())
        .run();
}

fn setup(world: &mut World) {
    let cube = Mesh::load(MeshType::Cube);
    let cube_handle = {
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        mesh_storage.add(cube)
    };

    world.insert(
        (),
        vec![
            // lights
            (
                Light::default(),
                LocalToWorld::identity(),
                Translation::new(4.0, -4.0, 5.0),
                Rotation::from_euler_angles(0.0, 0.0, 0.0),
            ),
        ],
    );

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
                    Vec3::new(6.0, -40.0, 20.0),
                    Vec3::new(0.0, 0.0, 0.0),
                    Vec3::new(0.0, 0.0, 1.0),
                )),
            ),
        ],
    );

    let mut rng = StdRng::from_entropy();
    for _ in 0..70000 {
        create_person(
            world,
            cube_handle.clone(),
            Translation::new(rng.gen_range(-50.0, 50.0), 0.0, rng.gen_range(-50.0, 50.0)),
        );
    }
}

fn build_wander_system() -> Box<dyn Schedulable> {
    let mut rng = StdRng::from_entropy();

    SystemBuilder::new("Wander")
        .read_resource::<Time>()
        .with_query(<(
            Read<Person>,
            Read<Translation>,
            Write<Wander>,
            Write<NavigationPoint>,
        )>::query())
        .build(move |_, world, time, person_query| {
            for (_, translation, mut wander, mut navigation_point) in person_query.iter_mut(world) {
                wander.elapsed += time.delta_seconds;
                if wander.elapsed >= wander.duration {
                    let direction = math::vec3(
                        rng.gen_range(-1.0, 1.0),
                        rng.gen_range(-1.0, 1.0),
                        rng.gen_range(0.0, 0.001),
                    )
                    .normalize();
                    let distance =
                        rng.gen_range(wander.distance_bounds.x(), wander.distance_bounds.y());
                    navigation_point.target = translation.0 + direction * distance;
                    wander.elapsed = 0.0;
                    wander.duration =
                        rng.gen_range(wander.duration_bounds.x(), wander.duration_bounds.y());
                }
            }
        })
}

fn build_navigate_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("Navigate")
        .with_query(<(
            Read<Person>,
            Write<Translation>,
            Write<Velocity>,
            Write<NavigationPoint>,
        )>::query())
        .build(move |_, world, _, person_query| {
            for (_, translation, mut velocity, navigation_point) in person_query.iter_mut(world) {
                let distance = navigation_point.target - translation.0;
                if distance.length() > 0.01 {
                    let direction = distance.normalize();
                    velocity.value = direction * 2.0;
                } else {
                    velocity.value = math::vec3(0.0, 0.0, 0.0);
                }
            }
        })
}

fn build_move_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("Move")
        .read_resource::<Time>()
        .with_query(<(Write<Translation>, Read<Velocity>)>::query())
        .build(move |_, world, time, person_query| {
            for (mut translation, velocity) in person_query.iter_mut(world) {
                translation.0 += velocity.value * time.delta_seconds;
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
        .with_query(<(Read<Person>,)>::query())
        .build(move |_, world, time, person_query| {
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
                println!("peeps: {}", person_query.iter(world).count());
                elapsed = 0.0;
            }
        })
}

fn create_person(world: &mut World, mesh_handle: Handle<Mesh>, translation: Translation) {
    world.insert(
        (),
        vec![(
            Person {},
            Wander {
                duration_bounds: math::vec2(3.0, 10.0),
                distance_bounds: math::vec2(-50.0, 50.0),
                elapsed: 0.0,
                duration: 0.0,
            },
            NavigationPoint {
                target: math::vec3(0.0, 0.0, 0.0),
            },
            Velocity {
                value: math::vec3(0.0, 0.0, 0.0),
            },
            Instanced,
            StandardMaterial {
                albedo: (math::vec4(0.5, 0.3, 0.3, 1.0) * random::<f32>()).into(),
            },
            mesh_handle,
            LocalToWorld::identity(),
            translation,
        )],
    );
}
