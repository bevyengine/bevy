use bevy::prelude::*;

/// This example illustrates how to create parent->child relationships between entities how parent transforms
/// are propagated to their descendants
fn main() {
    App::build()
        .add_resource(Msaa { samples: 4 })
        .add_default_plugins()
        .add_startup_system(setup.system())
        .add_system(rotator_system.system())
        .add_system(print_children.system())
        .run();
}

/// this component indicates what entities should rotate
struct Rotator;

/// rotates the parent, which will result in the child also rotating
fn rotator_system(time: Res<Time>, mut query: Query<(&Rotator, &mut Transform)>) {
    for (_rotator, mut transform) in &mut query.iter() {
        transform.rotate(&Rotation(Quat::from_rotation_x(3.0 * time.delta_seconds)));
    }
}

fn print_children(
    mut root_query: Query<(&Rotator, &Transform)>,
    mut child_query: Query<(&Parent, &Transform)>,
) {
    for (_, transform) in root_query.iter().iter() {
        println!("parent: {:?}", transform.global_matrix());
    }

    for (i, (_, transform)) in child_query.iter().iter().enumerate() {
        println!("child {}: {:?}", i, transform.global_matrix());
    }
}

/// set up a simple scene with a "parent" cube and a "child" cube
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let cube_material_handle = materials.add(StandardMaterial {
        albedo: Color::rgb(0.5, 0.4, 0.3),
        ..Default::default()
    });

    commands
        // parent cube
        .spawn(PbrComponents {
            mesh: cube_handle,
            material: cube_material_handle,
            transform: Translation::new(0.0, 0.0, 1.0).into(),
            ..Default::default()
        })
        .with(Rotator)
        .with_children(|parent| {
            // child cube
            parent.spawn(PbrComponents {
                mesh: cube_handle,
                material: cube_material_handle,
                transform: Translation::new(0.0, 0.0, 3.0).into(),
                ..Default::default()
            });
        })
        .with_children(|parent| {
            // child cube
            parent.spawn(PbrComponents {
                mesh: cube_handle,
                material: cube_material_handle,
                transform: Translation::new(3.0, 0.0, 0.0).into(),
                ..Default::default()
            });
        })
        // light
        .spawn(LightComponents {
            transform: Translation::new(4.0, 5.0, -4.0).into(),
            ..Default::default()
        })
        // camera
        .spawn(Camera3dComponents {
            transform: Transform::new(Mat4::face_toward(
                Vec3::new(5.0, 10.0, 10.0),
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(0.0, 1.0, 0.0),
            )),
            ..Default::default()
        });
}
