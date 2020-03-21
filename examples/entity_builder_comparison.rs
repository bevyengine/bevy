use bevy::prelude::*;

fn main() {
    App::build().add_defaults().setup_world(setup).run();
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
            plane_handle,
            StandardMaterial {
                albedo: Color::rgb(0.1, 0.2, 0.1),
                ..Default::default()
            },
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, 0.0),
        )],
    );

    // cube
    world.insert(
        (),
        vec![(
            cube_handle,
            StandardMaterial {
                albedo: Color::rgb(0.5, 0.3, 0.3),
                ..Default::default()
            },
            LocalToWorld::identity(),
            Translation::new(0.0, 0.0, 1.0),
        )],
    );

    // light
    world.insert(
        (),
        vec![(
            Light::default(),
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
        .add(plane_handle)
        .add(StandardMaterial {
            albedo: Color::rgb(0.1, 0.2, 0.1),
            ..Default::default()
        })
        .add(LocalToWorld::identity())
        .add(Translation::new(0.0, 0.0, 0.0))
        // cube
        .build_entity()
        .add(cube_handle)
        .add(StandardMaterial {
            albedo: Color::rgb(0.5, 0.3, 0.3),
            ..Default::default()
        })
        .add(LocalToWorld::identity())
        .add(Translation::new(0.0, 0.0, 1.0))
        // light
        .build_entity()
        .add(Light::default())
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
    plane_material_handle: Handle<StandardMaterial>,
    cube_handle: Handle<Mesh>,
    cube_material_handle: Handle<StandardMaterial>,
) {
    world
        .build()
        // plane
        .add_entity(MeshEntity {
            mesh: plane_handle,
            material: plane_material_handle,
            ..Default::default()
        })
        // cube
        .add_entity(MeshEntity {
            mesh: cube_handle,
            material: cube_material_handle,
            ..Default::default()
        })
        // light
        .add_entity(LightEntity {
            translation: Translation::new(4.0, -4.0, 5.0),
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
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
        })
        .build();
}

fn setup(world: &mut World, resources: &mut Resources) {
    let mut mesh_storage = resources.get_mut::<AssetStorage<Mesh>>().unwrap();
    let mut material_storage = resources
        .get_mut::<AssetStorage<StandardMaterial>>()
        .unwrap();
    let cube_handle = mesh_storage.add(Mesh::load(MeshType::Cube));
    let plane_handle = mesh_storage.add(Mesh::load(MeshType::Plane { size: 10.0 }));

    let cube_material_handle = material_storage.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.3, 0.3),
        ..Default::default()
    });
    let plane_material_handle = material_storage.add(StandardMaterial {
        albedo: Color::rgb(0.1, 0.2, 0.1),
        ..Default::default()
    });

    // no-archetype precompile: 1.24 sec
    // archetype precompile: 1.07 sec
    // create_entities_insert_vec(world, plane_handle, cube_handle);

    // no-archetype precompile: .93
    // no-archetype precompile: .93
    // create_entities_builder_add_component(world, plane_handle, cube_handle);

    // archetype precompile: 0.65
    create_entities_builder_archetype(
        world,
        plane_handle,
        plane_material_handle,
        cube_handle,
        cube_material_handle,
    );
}
