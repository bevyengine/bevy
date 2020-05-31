use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

/// set up a simple scene
fn setup(
    command_buffer: &mut CommandBuffer,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // create a cube and a plane mesh
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let plane_handle = meshes.add(Mesh::from(shape::Plane { size: 10.0 }));

    // create materials for our cube and plane
    let cube_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });
    let plane_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.1, 0.2, 0.1),
        ..Default::default()
    });

    // add entities to the world
    command_buffer
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
            translation: Translation::new(0.0, 0.0, 1.0),
            ..Default::default()
        })
        // light
        .add_entity(LightEntity {
            translation: Translation::new(4.0, -4.0, 5.0),
            ..Default::default()
        })
        // camera
        .add_entity(PerspectiveCameraEntity {
            local_to_world: LocalToWorld::new_sync_disabled(Mat4::look_at_rh(
                Vec3::new(3.0, 8.0, 5.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 0.0, 1.0),
            )),
            ..Default::default()
        });
}
