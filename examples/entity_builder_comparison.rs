use bevy::prelude::*;

fn main() {
    AppBuilder::new()
        .add_defaults()
        .setup_world(setup)
        .run();
}

#[allow(dead_code)]
fn create_entities_insert_vec(
    world: &mut World,
    plane_handle: Handle<Mesh>,
    cube_handle: Handle<Mesh>,
) {
    // plane
    world.insert(
        (),
        vec![(
            plane_handle.clone(),
            Material::new(Albedo::Color(math::vec4(0.1, 0.2, 0.1, 1.0))),
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, 0.0),
        )],
    );

    // cube
    world.insert(
        (),
        vec![(
            cube_handle,
            Material::new(Albedo::Color(math::vec4(0.5, 0.3, 0.3, 1.0))),
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
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
        )],
    );
}

#[allow(dead_code)]
fn create_entities_builder_add_component(
    world: &mut World,
    plane_handle: Handle<Mesh>,
    cube_handle: Handle<Mesh>,
) {
    world
        .build()
        // plane
        .build_entity()
        .add(plane_handle.clone())
        .add(Material::new(Albedo::Color(math::vec4(0.1, 0.2, 0.1, 1.0))))
        .add(LocalToWorld::identity())
        .add(Translation::new(0.0, 0.0, 0.0))
        // cube
        .build_entity()
        .add(cube_handle)
        .add(Material::new(Albedo::Color(math::vec4(0.5, 0.3, 0.3, 1.0))))
        .add(LocalToWorld::identity())
        .add(Translation::new(0.0, 0.0, 1.0))
        // light
        .build_entity()
        .add(Light {
            color: wgpu::Color {
                r: 0.8,
                g: 0.8,
                b: 0.5,
                a: 1.0,
            },
            fov: f32::to_radians(60.0),
            depth: 0.1..50.0,
            target_view: None,
        })
        .add(LocalToWorld::identity())
        .add(Translation::new(4.0, -4.0, 5.0))
        .add(Rotation::from_euler_angles(0.0, 0.0, 0.0))
        // camera
        .build_entity()
        .add(Camera::new(CameraType::Projection {
            fov: std::f32::consts::PI / 4.0,
            near: 1.0,
            far: 1000.0,
            aspect_ratio: 1.0,
        }))
        .add(ActiveCamera)
        .add(LocalToWorld(Mat4::look_at_rh(
            Vec3::new(3.0, 8.0, 5.0),
            Vec3::new(0.0, 0.0, 0.0),
            Vec3::new(0.0, 0.0, 1.0),
        )))
        .build();
}

fn create_entities_builder_archetype(
    world: &mut World,
    plane_handle: Handle<Mesh>,
    cube_handle: Handle<Mesh>,
) {
    world
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
            mesh: cube_handle,
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
        })
        .build();
}

fn setup(world: &mut World) {
    let cube = Mesh::load(MeshType::Cube);
    let plane = Mesh::load(MeshType::Plane { size: 10.0 });

    let (cube_handle, plane_handle) = {
        let mut mesh_storage = world.resources.get_mut::<AssetStorage<Mesh>>().unwrap();
        (mesh_storage.add(cube), mesh_storage.add(plane))
    };

    // no-archetype precompile: 1.24 sec
    // archetype precompile: 1.07 sec
    // create_entities_insert_vec(world, plane_handle, cube_handle);

    // no-archetype precompile: .93
    // noarchetype precompile: .93
    // create_entities_builder_add_component(world, plane_handle, cube_handle);

    // archetype precompile: 0.65
    create_entities_builder_archetype(world, plane_handle, cube_handle);
}
