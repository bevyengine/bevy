use bevy::{*, render::*, asset::{Asset, AssetStorage}, math::{Mat4, Quat, Vec3}, Schedulable, Parent};

struct Rotator;

fn main() {
    AppBuilder::new()
        .add_defaults()
        .setup(&setup)
        .run();
}

fn build_rotator_system() -> Box<dyn Schedulable> {
    SystemBuilder::new("Rotator")
        .read_resource::<Time>()
        .with_query(<(
            Write<Rotator>,
            Write<Rotation>,
        )>::query())
        .build(move |_, world, time , light_query| {
            for (_, mut rotation) in light_query.iter(world) {
                rotation.0 = rotation.0 * Quat::from_rotation_x(3.0 * time.delta_seconds);
            }
        })
}

fn setup(world: &mut World, scheduler: &mut SystemScheduler<AppStage>) {
    let cube = Mesh::load(MeshType::Cube);
    let plane = Mesh::load(MeshType::Plane{ size: 10.0 });

    let (cube_handle, plane_handle) = {
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh, MeshType>>().unwrap();
        (mesh_storage.add(cube, "cube"), mesh_storage.add(plane, "plane"))
    };

    let transform_system_bundle = transform_system_bundle::build(world);
    scheduler.add_systems(AppStage::Update, transform_system_bundle);
    scheduler.add_system(AppStage::Update, build_rotator_system());

    // plane
    world.insert((), vec![
        (
            plane_handle.clone(),
            Material::new(math::vec4(0.1, 0.2, 0.1, 1.0)),
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, -5.0)
        ),
    ]);
    
    // cube
    let parent_cube = *world.insert((), vec![
        (
            cube_handle.clone(),
            Material::new(math::vec4(0.5, 0.3, 0.3, 1.0)),
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, 1.0),
            Rotation::from_euler_angles(0.0, 0.0, 0.0),
            Rotator,
        )
    ]).first().unwrap();

    // cube
    world.insert((), vec![
        (
            cube_handle,
            Material::new(math::vec4(0.5, 0.3, 0.3, 1.0)),
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, 3.0),
            Parent(parent_cube),
            LocalToParent::identity(),
        )
    ]);

    // light
    world.insert((), vec![
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
            LocalToWorld::identity(),
            Translation::new(4.0, -4.0, 5.0),
            Rotation::from_euler_angles(0.0, 0.0, 0.0)
        ),
    ]);

    // camera
    world.insert((), vec![
        (
            Camera::new(CameraType::Projection {
                fov: std::f32::consts::PI / 4.0,
                near: 1.0,
                far: 1000.0,
                aspect_ratio: 1.0,
            }),
            ActiveCamera,
            LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, -15.0, 8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),)),
        )
    ]);
}