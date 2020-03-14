use bevy::prelude::*;
use rand::{random, rngs::StdRng, Rng, SeedableRng};

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
        .add_system(bevy::diagnostics::build_fps_printer_system())
        .run();
}

fn setup(world: &mut World, resources: &mut Resources) {
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let cube_handle = mesh_storage.add(Mesh::load(MeshType::Cube));

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
            cube_handle,
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
            StandardMaterial {
                albedo: (math::vec4(0.5, 0.3, 0.3, 1.0) * random::<f32>()).into(),
            },
            Renderable {
                instanced: true,
                ..Default::default()
            },
            mesh_handle,
            LocalToWorld::identity(),
            translation,
        )],
    );
}
