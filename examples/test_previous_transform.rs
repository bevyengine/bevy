//! Test example that verifies PreviousGlobalTransform is correctly initialized
//! to match GlobalTransform on the first frame for new entities.

use bevy::pbr::PreviousGlobalTransform;
use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, check_transforms)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(3.0, 3.0, 3.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Light
    commands.spawn((
        DirectionalLight::default(),
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.5, -0.5, 0.0)),
    ));

    // Spawn a mesh entity - this should trigger PreviousGlobalTransform initialization
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(1.0))),
        MeshMaterial3d(materials.add(StandardMaterial::default())),
        Transform::from_xyz(1.0, 2.0, 3.0),
        TestEntity,
    ));
}

#[derive(Component)]
struct TestEntity;

fn check_transforms(
    query: Query<(&GlobalTransform, Option<&PreviousGlobalTransform>), With<TestEntity>>,
    mut frame_count: Local<u32>,
) {
    *frame_count += 1;

    for (global_transform, maybe_previous) in &query {
        println!(
            "Frame {}: GlobalTransform: {:?}",
            *frame_count,
            global_transform.translation()
        );
        if let Some(previous) = maybe_previous {
            println!(
                "Frame {}: PreviousGlobalTransform exists: {:?}",
                *frame_count, previous.0.translation
            );

            // On the first frame, previous should equal current
            if *frame_count == 1 {
                let current_affine = global_transform.affine();
                if (previous.0.translation - current_affine.translation).length() < 0.001 {
                    println!(
                        "SUCCESS: PreviousGlobalTransform correctly initialized to GlobalTransform"
                    );
                } else {
                    println!("FAIL: PreviousGlobalTransform not initialized correctly");
                    println!("Current: {:?}", current_affine.translation);
                    println!("Previous: {:?}", previous.0.translation);
                }
            }
        } else {
            println!("Frame {}: PreviousGlobalTransform missing", *frame_count);
        }
    }

    // Exit after a few frames
    if *frame_count >= 3 {
        std::process::exit(0);
    }
}
