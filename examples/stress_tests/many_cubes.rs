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
    core_pipeline::prepass::DepthPrepass,
    diagnostic::{DiagnosticsStore, FrameTimeDiagnosticsPlugin},
    math::{DVec2, DVec3},
    pbr::NotShadowCaster,
    prelude::*,
    render::render_resource::Face,
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
        ))
        .add_systems(Startup, setup)
        .add_systems(Update, (move_camera, update_state))
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

    // Initialized below
    let camera_entity;

    match std::env::args().nth(1).as_deref() {
        Some("cube") => {
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
            camera_entity = commands
                .spawn(Camera3dBundle {
                    transform: Transform::from_xyz(WIDTH as f32, HEIGHT as f32, WIDTH as f32),
                    ..default()
                })
                .id();
        }
        _ => {
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
            camera_entity = commands.spawn(Camera3dBundle::default()).id();
        }
    }

    let depth_prepass_enabled = std::env::args().nth(2).as_deref() == Some("prepass-enabled");
    if depth_prepass_enabled {
        commands.entity(camera_entity).insert(DepthPrepass);
    }

    commands.spawn((
        PbrBundle {
            mesh: mesh.clone_weak(),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                double_sided: true,
                cull_mode: Some(Face::Front),
                ..default()
            }),
            transform: Transform {
                translation: Vec3::new(0.0, HEIGHT as f32 * 2.5, 0.0),
                scale: Vec3::splat(WIDTH as f32 * 5.0),
                ..default()
            },
            ..default()
        },
        NotShadowCaster,
    ));

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

    let directional_light_shadows_enabled =
        std::env::args().nth(3).as_deref() == Some("shadows-enabled");
    commands.spawn(DirectionalLightBundle {
        directional_light: DirectionalLight {
            shadows_enabled: directional_light_shadows_enabled,
            ..default()
        },
        ..default()
    });

    let style = TextStyle {
        font_size: 32.0,
        ..default()
    };
    commands.spawn((
        TextBundle::from_sections([
            TextSection::new(depth_prepass_string(depth_prepass_enabled), style.clone()),
            TextSection::new(
                directional_light_shadows_string(directional_light_shadows_enabled),
                style.clone(),
            ),
            TextSection::new(mesh_stats_string(0, 0), style.clone()),
            TextSection::new(frame_stats_string(0.0, 0.0), style),
        ]),
        UiState,
    ));
}

#[derive(Component)]
struct UiState;

fn toggle_state_string(v: bool) -> &'static str {
    if v {
        "enabled"
    } else {
        "disabled"
    }
}

fn depth_prepass_string(depth_prepass_enabled: bool) -> String {
    format!(
        "Depth prepass: {}               (toggle by pressing 'p')\n",
        toggle_state_string(depth_prepass_enabled)
    )
}

fn directional_light_shadows_string(directional_light_shadows_enabled: bool) -> String {
    format!(
        "Directional light shadows: {}   (toggle by pressing 's')\n",
        toggle_state_string(directional_light_shadows_enabled)
    )
}

fn mesh_stats_string(total_meshes: usize, visible_meshes: usize) -> String {
    format!(
        "Meshes: {}\nVisible Meshes {} ({:5.1}%)\n",
        total_meshes,
        visible_meshes,
        100.0 * (visible_meshes as f32 / total_meshes as f32),
    )
}

fn frame_stats_string(fps: f32, frame_time_ms: f32) -> String {
    format!(
        "Frame rate: {:>6.1} fps\nFrame time: {:>6.3} ms\n",
        fps, frame_time_ms
    )
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

fn update_state(
    mut commands: Commands,
    keys: Res<Input<KeyCode>>,
    mut directional_lights: Query<&mut DirectionalLight>,
    cameras_no_prepass: Query<Entity, (With<Camera>, Without<DepthPrepass>)>,
    cameras_prepass: Query<Entity, (With<Camera>, With<DepthPrepass>)>,
    diagnostics: Res<DiagnosticsStore>,
    mut query: Query<&mut Text, With<UiState>>,
    meshes: Query<(&Handle<Mesh>, &ComputedVisibility)>,
) {
    // Update directional light shadow state
    let mut shadows_string = None;
    if keys.just_pressed(KeyCode::S) {
        let mut directional_light_shadows_enabled = false;
        for mut light in &mut directional_lights {
            light.shadows_enabled = !light.shadows_enabled;
            directional_light_shadows_enabled = light.shadows_enabled;
        }
        shadows_string = Some(directional_light_shadows_string(
            directional_light_shadows_enabled,
        ));
    }

    // Update depth prepass state
    let mut prepass_string = None;
    if keys.just_pressed(KeyCode::P) {
        let mut depth_prepass_enabled = false;
        for entity in &cameras_no_prepass {
            commands.entity(entity).insert(DepthPrepass);
            depth_prepass_enabled = true;
        }
        for entity in &cameras_prepass {
            commands.entity(entity).remove::<DepthPrepass>();
            depth_prepass_enabled = false;
        }
        prepass_string = Some(depth_prepass_string(depth_prepass_enabled));
    }

    // Update the UI
    let fps = diagnostics
        .get(FrameTimeDiagnosticsPlugin::FPS)
        .and_then(|fps| fps.smoothed())
        .unwrap_or_default() as f32;
    let frame_time_ms = diagnostics
        .get(FrameTimeDiagnosticsPlugin::FRAME_TIME)
        .and_then(|frame_time_ms| frame_time_ms.smoothed())
        .unwrap_or_default() as f32;
    let frame_stats_string = frame_stats_string(fps, frame_time_ms);

    let total_meshes = meshes.iter().len();
    let visible_meshes = meshes.iter().filter(|(_, cv)| cv.is_visible()).count();
    let mesh_stats_string = mesh_stats_string(total_meshes, visible_meshes);

    for mut text in &mut query {
        if let Some(shadows_string) = shadows_string.as_ref() {
            text.sections[0].value = shadows_string.clone();
        }
        if let Some(prepass_string) = prepass_string.as_ref() {
            text.sections[1].value = prepass_string.clone();
        }
        text.sections[2].value = mesh_stats_string.clone();
        text.sections[3].value = frame_stats_string.clone();
    }
}
