use bevy::*;
use bevy::{render::*, asset::{Asset, AssetStorage, Handle}, math::{Mat4, Quat, Vec3}, Schedulable, Parent};
use rand::{rngs::StdRng, Rng, SeedableRng, random};

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
        .build(move |
            _, world,
            time ,
            person_query| {
            for (_, translation, mut wander, mut navigation_point) in person_query.iter(world) {
                wander.elapsed += time.delta_seconds;
                if wander.elapsed >= wander.duration {
                    let direction = math::vec3(
                        rng.gen_range(-1.0, 1.0),
                        rng.gen_range(-1.0, 1.0),
                        rng.gen_range(0.0, 0.001),
                    ).normalize();
                    let distance = rng.gen_range(wander.distance_bounds.x(), wander.distance_bounds.y());
                    navigation_point.target = translation.0 + direction * distance;
                    wander.elapsed = 0.0;
                    wander.duration = rng.gen_range(wander.duration_bounds.x(), wander.duration_bounds.y());
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
        .build(move |
            _, world,
            _, person_query| {
            for (_, translation, mut velocity, navigation_point) in person_query.iter(world) {
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
        .with_query(<(
            Write<Translation>,
            Read<Velocity>,
        )>::query())
        .build(move |_, world, time , person_query| {
            for (mut translation, velocity) in person_query.iter(world) {
                translation.0 += velocity.value * time.delta_seconds;
            }
        })
}

fn build_print_status_system() -> Box<dyn Schedulable> {
    let mut elapsed = 0.0;
    SystemBuilder::new("PrintStatus")
        .read_resource::<Time>()
        .with_query(<(
            Read<Person>,
        )>::query())
        .build(move |_, world, time , person_query| {
            elapsed += time.delta_seconds;
            if elapsed > 1.0 {
                println!("fps: {}", if time.delta_seconds == 0.0 { 0.0 } else { 1.0 / time.delta_seconds });
                println!("peeps: {}", person_query.iter(world).count());
                elapsed = 0.0;
            }
        })
}

fn build_spawner_system(world: &mut World) -> Box<dyn Schedulable> {
    let mesh_handle = {
        let mesh_storage = world.resources.get_mut::<AssetStorage<Mesh, MeshType>>().unwrap();
        mesh_storage.get_named("cube").unwrap()
    };

    let duration = 10000.0;
    let mut elapsed = duration;
    let batch_size = 5;

    SystemBuilder::new("Spawner")
        .read_resource::<Time>()
        .with_query(<(
            Read<Person>,
        )>::query())
        .build(move |command_buffer, _, time , _| {
            elapsed += time.delta_seconds;
            if elapsed > duration {
                for _ in 0..batch_size {
                    spawn_person(command_buffer, mesh_handle.clone());
                }
                elapsed = 0.0;
            }
        })
}

fn build_light_rotator_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("LightRotator")
        .read_resource::<Time>()
        .with_query(<(
            Write<Light>,
            Write<Rotation>,
        )>::query())
        .build(move |_, world, time , light_query| {
            for (_, mut rotation) in light_query.iter(world) {
                rotation.0 = rotation.0 * Quat::from_rotation_x(3.0 * time.delta_seconds);
            }
        })
}

struct Person {
}

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
    let universe = Universe::new();
    let mut world = universe.create_world();
    let mut scheduler = SystemScheduler::<ApplicationStage>::new();

    let cube = Mesh::load(MeshType::Cube);
    let plane = Mesh::load(MeshType::Plane{ size: 25 });
    let mut mesh_storage = AssetStorage::<Mesh, MeshType>::new();

    let _cube_handle = mesh_storage.add(cube, "cube");
    let plane_handle = mesh_storage.add(plane, "plane");
    world.resources.insert(mesh_storage);

    let transform_system_bundle = transform_system_bundle::build(&mut world);
    scheduler.add_systems(ApplicationStage::Update, transform_system_bundle);
    scheduler.add_system(ApplicationStage::Update, build_wander_system());
    scheduler.add_system(ApplicationStage::Update, build_navigate_system());
    scheduler.add_system(ApplicationStage::Update, build_move_system());
    scheduler.add_system(ApplicationStage::Update, build_light_rotator_system());
    scheduler.add_system(ApplicationStage::Update, build_spawner_system(&mut world));
    scheduler.add_system(ApplicationStage::Update, build_print_status_system());

    world.insert((), vec![
        // plane
        (
            Material::new(math::vec4(0.1, 0.2, 0.1, 1.0)),
            plane_handle.clone(),
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, 0.0)
        ),
    ]);

    let x = *world.insert((), vec![
        // lights
        (
            Light {
                color: wgpu::Color {
                    r: 0.8,
                    g: 0.8,
                    b: 0.5,
                    a: 1.0,
                },
                fov: f32::to_radians(60.0),
                depth: 0.1 .. 50.0,
                target_view: None,
            },
            Material::new(math::vec4(0.5, 0.3, 0.3, 1.0)),
            // _cube_handle.clone(),
            LocalToWorld::identity(),
            Translation::new(4.0, -4.0, 5.0),
            Rotation::from_euler_angles(0.0, 0.0, 0.0)
        ),
        // (
        //     Light {
        //         color: wgpu::Color {
        //             r: 1.0,
        //             g: 0.5,
        //             b: 0.5,
        //             a: 1.0,
        //         },
        //         fov: f32::to_radians(45.0),
        //         depth: 1.0 .. 20.0,
        //         target_view: None,
        //     },
        //     // Material::new(math::vec4(0.5, 0.3, 0.3, 1.0) * random::<f32>()),
        //     // cube_handle.clone(),
        //     LocalToWorld::identity(),
        //     Translation::new(-5.0, 7.0, 10.0)
        // ),
    ]).first().unwrap();

    world.insert((), vec![
        (
            Material::new(math::vec4(1.0, 1.0, 1.0, 1.0)),
            _cube_handle.clone(),
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, 3.0),
            Scale(1.0),
            Parent(x),
            LocalToParent::identity(),
        )
    ]);

    world.insert((), vec![
        
        // camera
        (
            Camera::new(CameraType::Projection {
                fov: std::f32::consts::PI / 4.0,
                near: 1.0,
                far: 1000.0,
                aspect_ratio: 1.0,
            }),
            LocalToWorld(Mat4::look_at_rh(
                Vec3::new(6.0, -40.0, 20.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),)),
            // Translation::new(0.0, 0.0, 0.0),
        )
    ]);

    Application::run(universe, world, scheduler);
}

fn spawn_person(command_buffer: &mut CommandBuffer, mesh_handle: Handle<Mesh>) {
    command_buffer.insert((), vec![
        (
            Person{},
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
            Material::new(math::vec4(0.5, 0.3, 0.3, 1.0) * random::<f32>()),
            mesh_handle.clone(),
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, 1.0)
        ),
    ]);
}