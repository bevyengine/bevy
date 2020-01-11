use bevy::{*, render::*, asset::{Asset, AssetStorage}, math::{Mat4, Vec3}};

fn main() {
    AppBuilder::new()
        .add_defaults()
        .setup(&setup)
        .run();
}

fn setup(world: &mut World, scheduler: &mut SystemScheduler<AppStage>) {
    let cube = Mesh::load(MeshType::Cube);
    let cube_handle = {
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh, MeshType>>().unwrap();
        mesh_storage.add(cube, "cube")
    };

    let transform_system_bundle = transform_system_bundle::build(world);
    scheduler.add_systems(AppStage::Update, transform_system_bundle);

    world.insert((), vec![
        (
            cube_handle,
            LocalToWorld::identity(),
            Material::new(math::vec4(0.5, 0.3, 0.3, 1.0)),
        )
    ]);

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
            LocalToWorld::identity(),
            Translation::new(4.0, -4.0, 5.0),
            Rotation::from_euler_angles(0.0, 0.0, 0.0)
        ),
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
            ActiveCamera,
            LocalToWorld(Mat4::look_at_rh(
                Vec3::new(3.0, -5.0, 3.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),)),
        )
    ]);

    world.insert((), vec![
        // camera
        (
            Camera::new(CameraType::Orthographic {
                left: 0.0,
                right: 0.0,
                bottom: 0.0,
                top: 0.0,
                near: 0.0,
                far: 1.0,
            }),
            ActiveCamera2d,
        )
    ]);

    world.insert((), vec![
        (
            Rect {
                position: math::vec2(75.0, 75.0),
                dimensions: math::vec2(100.0, 100.0),
                color: math::vec4(0.0, 1.0, 0.0, 1.0),
            },
        )
    ]);

    world.insert((), vec![
        (
            Rect {
                position: math::vec2(50.0, 50.0),
                dimensions: math::vec2(100.0, 100.0),
                color: math::vec4(1.0, 0.0, 0.0, 1.0),
            },
        )
    ]);

    world.insert((), vec![
        (
            Rect {
                position: math::vec2(100.0, 100.0),
                dimensions: math::vec2(100.0, 100.0),
                color: math::vec4(0.0, 0.0, 1.0, 1.0),
            },
        )
    ]);
}