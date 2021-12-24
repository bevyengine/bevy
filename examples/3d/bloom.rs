use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .insert_resource(BloomSettings {
            enabled: true,
            threshold: 1.0,
            knee: 0.1,
            up_sample_scale: 1.0,
        })
        .add_startup_system(setup)
        .add_system(bounce)
        .run();
}

#[derive(Component)]
struct Bouncing;

/// set up a simple 3D scene
fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mesh = meshes.add(
        shape::Icosphere {
            radius: 0.5,
            subdivisions: 5,
        }
        .into(),
    );

    let material = materials.add(StandardMaterial {
        emissive: Color::rgb_linear(1.0, 0.3, 0.2) * 4.0,
        ..Default::default()
    });

    for x in -10..10 {
        for z in -10..10 {
            commands
                .spawn_bundle(PbrBundle {
                    mesh: mesh.clone(),
                    material: material.clone(),
                    transform: Transform::from_xyz(x as f32 * 2.0, 0.0, z as f32 * 2.0),
                    ..Default::default()
                })
                .insert(Bouncing);
        }
    }

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(-2.0, 2.5, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
        ..Default::default()
    });
}

fn bounce(time: Res<Time>, mut query: Query<&mut Transform, With<Bouncing>>) {
    for mut transform in query.iter_mut() {
        transform.translation.y = (transform.translation.x
            + transform.translation.z
            + time.seconds_since_startup() as f32)
            .sin();
    }
}
