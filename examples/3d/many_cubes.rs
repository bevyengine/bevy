use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugin(FrameTimeDiagnosticsPlugin::default())
        .add_plugin(LogDiagnosticsPlugin::default())
        .add_startup_system(setup)
        .add_system(move_camera)
        .add_system(print_mesh_count)
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    const WIDTH: usize = 200;
    const HEIGHT: usize = 200;
    let mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let material = materials.add(StandardMaterial {
        base_color: Color::PINK,
        ..default()
    });
    for x in 0..WIDTH {
        for y in 0..HEIGHT {
            // cube
            commands.spawn_bundle(PbrBundle {
                mesh: mesh.clone_weak(),
                material: material.clone_weak(),
                transform: Transform::from_xyz((x as f32) * 2.0, (y as f32) * 2.0, 0.0),
                ..default()
            });
            commands.spawn_bundle(PbrBundle {
                mesh: mesh.clone_weak(),
                material: material.clone_weak(),
                transform: Transform::from_xyz(
                    (x as f32) * 2.0,
                    HEIGHT as f32 * 2.0,
                    (y as f32) * 2.0,
                ),
                ..Default::default()
            });
            commands.spawn_bundle(PbrBundle {
                mesh: mesh.clone_weak(),
                material: material.clone_weak(),
                transform: Transform::from_xyz((x as f32) * 2.0, 0.0, (y as f32) * 2.0),
                ..Default::default()
            });
            commands.spawn_bundle(PbrBundle {
                mesh: mesh.clone_weak(),
                material: material.clone_weak(),
                transform: Transform::from_xyz(0.0, (x as f32) * 2.0, (y as f32) * 2.0),
                ..Default::default()
            });
        }
    }

    // add one cube, the only one with strong handles
    // also serves as a reference point during rotation
    commands.spawn_bundle(PbrBundle {
        mesh,
        material,
        transform: Transform {
            translation: Vec3::new(0.0, HEIGHT as f32 * 2.0, 0.0),
            scale: Vec3::splat(5.0),
            ..Default::default()
        },
        ..Default::default()
    });

    // camera
    commands.spawn_bundle(PerspectiveCameraBundle {
        transform: Transform::from_xyz(WIDTH as f32, HEIGHT as f32, WIDTH as f32),
        ..default()
    });

    commands.spawn_bundle(DirectionalLightBundle {
        ..Default::default()
    });
}

// System for rotating the camera
fn move_camera(time: Res<Time>, mut camera_query: Query<&mut Transform, With<Camera>>) {
    let mut camera_transform = camera_query.single_mut();
    camera_transform.rotate(Quat::from_rotation_z(time.delta_seconds() * 0.5));
    camera_transform.rotate(Quat::from_rotation_x(time.delta_seconds() * 0.5));
}

// System for printing the number of meshes on every tick of the timer
fn print_mesh_count(
    time: Res<Time>,
    mut timer: Local<PrintingTimer>,
    sprites: Query<(&Handle<Mesh>, &ComputedVisibility)>,
) {
    timer.0.tick(time.delta());

    if timer.0.just_finished() {
        info!(
            "Meshes: {} - Visible Meshes {}",
            sprites.iter().len(),
            sprites.iter().filter(|(_, cv)| cv.is_visible).count(),
        );
    }
}

struct PrintingTimer(Timer);

impl Default for PrintingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, true))
    }
}
