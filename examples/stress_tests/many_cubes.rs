//! Simple benchmark to test per-entity draw overhead.
//!
//! To measure performance realistically, be sure to run this in release mode.
//! `cargo run --example many_cubes --release`
//!
//! By default, this arranges the meshes in a cubical pattern, where the number of visible meshes
//! varies with the viewing angle. You can choose to run the demo with a spherical pattern that
//! distributes the meshes evenly.
//!
//! To start the demo using the spherical layout run
//! `cargo run --example many_cubes --release sphere`

use std::f64::consts::PI;

use bevy::{
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::{DVec2, DVec3},
    prelude::*,
    window::{PresentMode, WindowPlugin},
};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin,
            LogDiagnosticsPlugin::default(),
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_camera, print_mesh_count))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    warn!(include_str!("warning_string.txt"));

    const WIDTH: usize = 200;
    const HEIGHT: usize = 200;
    let mesh = meshes.add(Mesh::from(shape::Cube { size: 1.0 }));
    let material = materials.add(StandardMaterial {
        base_color: Color::PINK,
        ..default()
    });

    match std::env::args().nth(1).as_deref() {
        Some("sphere") => {
            // NOTE: This pattern is good for testing performance of culling as it provides roughly
            // the same number of visible meshes regardless of the viewing angle.
            const N_POINTS: usize = WIDTH * HEIGHT * 4;
            // NOTE: f64 is used to avoid precision issues that produce visual artifacts in the distribution
            let radius = WIDTH as f64 * 2.5;
            let golden_ratio = 0.5f64 * (1.0f64 + 5.0f64.sqrt());
            for i in 0..N_POINTS {
                let spherical_polar_theta_phi =
                    fibonacci_spiral_on_sphere(golden_ratio, i, N_POINTS);
                let unit_sphere_p = spherical_polar_to_cartesian(spherical_polar_theta_phi);
                commands.spawn(PbrBundle {
                    mesh: mesh.clone_weak(),
                    material: material.clone_weak(),
                    transform: Transform::from_translation((radius * unit_sphere_p).as_vec3()),
                    ..default()
                });
            }

            // camera
            commands.spawn(Camera3dBundle::default());
        }
        _ => {
            // NOTE: This pattern is good for demonstrating that frustum culling is working correctly
            // as the number of visible meshes rises and falls depending on the viewing angle.
            for x in 0..WIDTH {
                for y in 0..HEIGHT {
                    // introduce spaces to break any kind of moiré pattern
                    if x % 10 == 0 || y % 10 == 0 {
                        continue;
                    }
                    // cube
                    commands.spawn(PbrBundle {
                        mesh: mesh.clone_weak(),
                        material: material.clone_weak(),
                        transform: Transform::from_xyz((x as f32) * 2.5, (y as f32) * 2.5, 0.0),
                        ..default()
                    });
                    commands.spawn(PbrBundle {
                        mesh: mesh.clone_weak(),
                        material: material.clone_weak(),
                        transform: Transform::from_xyz(
                            (x as f32) * 2.5,
                            HEIGHT as f32 * 2.5,
                            (y as f32) * 2.5,
                        ),
                        ..default()
                    });
                    commands.spawn(PbrBundle {
                        mesh: mesh.clone_weak(),
                        material: material.clone_weak(),
                        transform: Transform::from_xyz((x as f32) * 2.5, 0.0, (y as f32) * 2.5),
                        ..default()
                    });
                    commands.spawn(PbrBundle {
                        mesh: mesh.clone_weak(),
                        material: material.clone_weak(),
                        transform: Transform::from_xyz(0.0, (x as f32) * 2.5, (y as f32) * 2.5),
                        ..default()
                    });
                }
            }
            // camera
            commands.spawn(Camera3dBundle {
                transform: Transform::from_xyz(WIDTH as f32, HEIGHT as f32, WIDTH as f32),
                ..default()
            });
        }
    }

    // add one cube, the only one with strong handles
    // also serves as a reference point during rotation
    commands.spawn(PbrBundle {
        mesh,
        material,
        transform: Transform {
            translation: Vec3::new(0.0, HEIGHT as f32 * 2.5, 0.0),
            scale: Vec3::splat(5.0),
            ..default()
        },
        ..default()
    });

    commands.spawn(DirectionalLightBundle { ..default() });
}

// NOTE: This epsilon value is apparently optimal for optimizing for the average
// nearest-neighbor distance. See:
// http://extremelearning.com.au/how-to-evenly-distribute-points-on-a-sphere-more-effectively-than-the-canonical-fibonacci-lattice/
// for details.
const EPSILON: f64 = 0.36;

fn fibonacci_spiral_on_sphere(golden_ratio: f64, i: usize, n: usize) -> DVec2 {
    DVec2::new(
        PI * 2. * (i as f64 / golden_ratio),
        (1.0 - 2.0 * (i as f64 + EPSILON) / (n as f64 - 1.0 + 2.0 * EPSILON)).acos(),
    )
}

fn spherical_polar_to_cartesian(p: DVec2) -> DVec3 {
    let (sin_theta, cos_theta) = p.x.sin_cos();
    let (sin_phi, cos_phi) = p.y.sin_cos();
    DVec3::new(cos_theta * sin_phi, sin_theta * sin_phi, cos_phi)
}

// System for rotating the camera
fn move_camera(time: Res<Time>, mut camera_query: Query<&mut Transform, With<Camera>>) {
    let mut camera_transform = camera_query.single_mut();
    let delta = time.delta_seconds() * 0.15;
    camera_transform.rotate_z(delta);
    camera_transform.rotate_x(delta);
}

// System for printing the number of meshes on every tick of the timer
fn print_mesh_count(
    time: Res<Time>,
    mut timer: Local<PrintingTimer>,
    sprites: Query<(&Handle<Mesh>, &ComputedVisibility)>,
) {
    timer.tick(time.delta());

    if timer.just_finished() {
        info!(
            "Meshes: {} - Visible Meshes {}",
            sprites.iter().len(),
            sprites.iter().filter(|(_, cv)| cv.is_visible()).count(),
        );
    }
}

#[derive(Deref, DerefMut)]
struct PrintingTimer(Timer);

impl Default for PrintingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}
