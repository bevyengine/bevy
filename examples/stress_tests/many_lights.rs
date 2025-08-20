//! Simple benchmark to test rendering many point lights.
//! Run with `WGPU_SETTINGS_PRIO=webgl2` to restrict to uniform buffers and max 256 lights.

use std::f64::consts::PI;

use bevy::{
    camera::ScalingMode,
    color::palettes::css::DEEP_PINK,
    diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin},
    math::{DVec2, DVec3},
    pbr::{ExtractedPointLight, GlobalClusterableObjectMeta},
    prelude::*,
    render::{Render, RenderApp, RenderSystems},
    window::{PresentMode, WindowResolution},
    winit::{UpdateMode, WinitSettings},
};
use rand::{rng, Rng};

fn main() {
    App::new()
        .add_plugins((
            DefaultPlugins.set(WindowPlugin {
                primary_window: Some(Window {
                    resolution: WindowResolution::new(1920, 1080).with_scale_factor_override(1.0),
                    title: "many_lights".into(),
                    present_mode: PresentMode::AutoNoVsync,
                    ..default()
                }),
                ..default()
            }),
            FrameTimeDiagnosticsPlugin::default(),
            LogDiagnosticsPlugin::default(),
            LogVisibleLights,
        ))
        .insert_resource(WinitSettings {
            focused_mode: UpdateMode::Continuous,
            unfocused_mode: UpdateMode::Continuous,
        })
        .add_systems(Startup, setup)
        .add_systems(Update, (move_camera, print_light_count))
        .run();
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    warn!(include_str!("warning_string.txt"));

    const LIGHT_RADIUS: f32 = 0.3;
    const LIGHT_INTENSITY: f32 = 1000.0;
    const RADIUS: f32 = 50.0;
    const N_LIGHTS: usize = 100_000;

    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(RADIUS).mesh().ico(9).unwrap())),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_scale(Vec3::NEG_ONE),
    ));

    let mesh = meshes.add(Cuboid::default());
    let material = materials.add(StandardMaterial {
        base_color: DEEP_PINK.into(),
        ..default()
    });

    // NOTE: This pattern is good for testing performance of culling as it provides roughly
    // the same number of visible meshes regardless of the viewing angle.
    // NOTE: f64 is used to avoid precision issues that produce visual artifacts in the distribution
    let golden_ratio = 0.5f64 * (1.0f64 + 5.0f64.sqrt());

    // Spawn N_LIGHTS many lights
    commands.spawn_batch((0..N_LIGHTS).map(move |i| {
        let mut rng = rng();

        let spherical_polar_theta_phi = fibonacci_spiral_on_sphere(golden_ratio, i, N_LIGHTS);
        let unit_sphere_p = spherical_polar_to_cartesian(spherical_polar_theta_phi);

        (
            PointLight {
                range: LIGHT_RADIUS,
                intensity: LIGHT_INTENSITY,
                color: Color::hsl(rng.random_range(0.0..360.0), 1.0, 0.5),
                ..default()
            },
            Transform::from_translation((RADIUS as f64 * unit_sphere_p).as_vec3()),
        )
    }));

    // camera
    match std::env::args().nth(1).as_deref() {
        Some("orthographic") => commands.spawn((
            Camera3d::default(),
            Projection::from(OrthographicProjection {
                scaling_mode: ScalingMode::FixedHorizontal {
                    viewport_width: 20.0,
                },
                ..OrthographicProjection::default_3d()
            }),
        )),
        _ => commands.spawn(Camera3d::default()),
    };

    // add one cube, the only one with strong handles
    // also serves as a reference point during rotation
    commands.spawn((
        Mesh3d(mesh),
        MeshMaterial3d(material),
        Transform {
            translation: Vec3::new(0.0, RADIUS, 0.0),
            scale: Vec3::splat(5.0),
            ..default()
        },
    ));
}

// NOTE: This epsilon value is apparently optimal for optimizing for the average
// nearest-neighbor distance. See:
// http://extremelearning.com.au/how-to-evenly-distribute-points-on-a-sphere-more-effectively-than-the-canonical-fibonacci-lattice/
// for details.
const EPSILON: f64 = 0.36;
fn fibonacci_spiral_on_sphere(golden_ratio: f64, i: usize, n: usize) -> DVec2 {
    DVec2::new(
        PI * 2. * (i as f64 / golden_ratio),
        ops::acos((1.0 - 2.0 * (i as f64 + EPSILON) / (n as f64 - 1.0 + 2.0 * EPSILON)) as f32)
            as f64,
    )
}

fn spherical_polar_to_cartesian(p: DVec2) -> DVec3 {
    let (sin_theta, cos_theta) = p.x.sin_cos();
    let (sin_phi, cos_phi) = p.y.sin_cos();
    DVec3::new(cos_theta * sin_phi, sin_theta * sin_phi, cos_phi)
}

// System for rotating the camera
fn move_camera(time: Res<Time>, mut camera_transform: Single<&mut Transform, With<Camera>>) {
    let delta = time.delta_secs() * 0.15;
    camera_transform.rotate_z(delta);
    camera_transform.rotate_x(delta);
}

// System for printing the number of meshes on every tick of the timer
fn print_light_count(time: Res<Time>, mut timer: Local<PrintingTimer>, lights: Query<&PointLight>) {
    timer.0.tick(time.delta());

    if timer.0.just_finished() {
        info!("Lights: {}", lights.iter().len());
    }
}

struct LogVisibleLights;

impl Plugin for LogVisibleLights {
    fn build(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.add_systems(
            Render,
            print_visible_light_count.in_set(RenderSystems::Prepare),
        );
    }
}

// System for printing the number of meshes on every tick of the timer
fn print_visible_light_count(
    time: Res<Time>,
    mut timer: Local<PrintingTimer>,
    visible: Query<&ExtractedPointLight>,
    global_light_meta: Res<GlobalClusterableObjectMeta>,
) {
    timer.0.tick(time.delta());

    if timer.0.just_finished() {
        info!(
            "Visible Lights: {}, Rendered Lights: {}",
            visible.iter().len(),
            global_light_meta.entity_to_index.len()
        );
    }
}

struct PrintingTimer(Timer);

impl Default for PrintingTimer {
    fn default() -> Self {
        Self(Timer::from_seconds(1.0, TimerMode::Repeating))
    }
}
