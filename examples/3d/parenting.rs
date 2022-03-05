use bevy::prelude::*;

/// This example illustrates how to create parent->child relationships between entities how parent
/// transforms are propagated to their descendants.
///  `Transform` can be inherited from any entity with both the `Transform` and `GlobalTransform` components.
fn main() {
    App::new()
        .insert_resource(Msaa { samples: 4 })
        .add_plugins(DefaultPlugins)
        .add_startup_system(setup)
        .add_system(rotator_system)
        .run();
}

/// This component indicates what entities should rotate.
#[derive(Component)]
struct Rotator;

/// Rotates the parent, which will result in the child also rotating.
fn rotator_system(time: Res<Time>, mut query: Query<&mut Transform, With<Rotator>>) {
    for mut transform in query.iter_mut() {
        transform.rotation *= Quat::from_rotation_x(3.0 * time.delta_seconds());
    }
}

/// Set up a simple scene with two hierarchies:
/// - a "parent" cube and a "child" cube
/// - an invisible "parent" (that has a transform, but not a mesh) and a "child" capsule
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let cube_handle = meshes.add(Mesh::from(shape::Cube { size: 2.0 }));
    let capsule_handle = meshes.add(Mesh::from(shape::Capsule::default()));
    let material_handle = materials.add(StandardMaterial {
        base_color: Color::BISQUE,
        ..default()
    });

    // parent cube
    commands
        .spawn_bundle(PbrBundle {
            mesh: cube_handle.clone(),
            material: material_handle.clone(),
            transform: Transform::from_xyz(-2.0, 0.0, 1.0),
            ..default()
        })
        .insert(Rotator)
        .with_children(|parent| {
            // child cube
            parent.spawn_bundle(PbrBundle {
                mesh: cube_handle,
                material: material_handle.clone(),
                transform: Transform::from_xyz(0.0, 0.0, 3.0),
                ..default()
            });
        });
    // invisible parent
    commands
        .spawn_bundle(TransformBundle {
            local: Transform::from_xyz(2.0, 0.0, 1.0),
            global: GlobalTransform::default(),
        })
        .insert(Rotator)
        .with_children(|parent| {
            // child capsule
            parent.spawn_bundle(PbrBundle {
                mesh: capsule_handle,
                material: material_handle,
                transform: Transform::from_xyz(0.0, 0.0, 3.0),
                ..default()
            });
        });
    // light
    commands.spawn_bundle(PointLightBundle {
        transform: Transform::from_xyz(4.0, 5.0, -4.0),
        ..default()
    });
    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(5.0, 10.0, 10.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..default()
    });
}
