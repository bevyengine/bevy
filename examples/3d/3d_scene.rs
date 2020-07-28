use bevy::prelude::*;

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(setup.system())
        .run();
}

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
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
    commands
        // plane
        .spawn(PbrComponents {
            mesh: plane_handle,
            material: plane_material_handle,
            ..Default::default()
        })
        // cube
        .spawn(PbrComponents {
            mesh: cube_handle,
            material: cube_material_handle,
            translation: Translation::new(0.0, 1.0, 0.0),
            ..Default::default()
        })
        // light
        .spawn(LightComponents {
            translation: Translation::new(4.0, 8.0, 4.0),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new_sync_disabled(Mat4::face_toward(
                Vec3::new(-3.0, 5.0, 8.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
