//! This example demonstrates retained gizmos.
//!
//! Normally, gizmos are immediate. That is, a gizmo must be drawn through a
//! ['Gizmos'] system parameter for each frame it is to be shown. A retained
//! gizmo is spawned as an entity and is rendered so long as it is alive and
//! visible.
//!
//! Retained gizmos are recommended for drawing a larger number of static elements.
//! They have a transform and visibility components for rendering control.
//!
//! This example spawns gizmos on a jittery delay, choosing parameters randomly
//! and implementing a cheap "twinkling" effect by toggling visibility randomly.
//! The count is kept between 25 and about 150.
//!
//! This example uses the free camera plugin for observing the scene. Controls
//! are printed to the console on startup.

use std::ops::Range;

use bevy::{
    camera_controller::free_camera::{FreeCamera, FreeCameraPlugin},
    color::palettes::css::*,
    prelude::*,
};
use rand::RngExt;

/// minimum gizmos to keep alive before rolling a change to despawn
const MIN_COUNT: usize = 25;
/// maximum gizmos to keep alive
const SOFT_PEAK: f64 = 150.0;
/// bounds period between spawns, in seconds
const DELAY_RANGE: Range<f32> = 1.2..3.0;
/// color choices for the spawned gizmos
const COLORS: [Srgba; 7] = [
    CORNSILK,
    DARK_ORANGE,
    MEDIUM_AQUAMARINE,
    CORAL,
    BISQUE,
    ANTIQUE_WHITE,
    BLUE_VIOLET,
];

// spawn bounds
const X_BOUNDS: Range<f32> = -31.0..31.0;
const Y_BOUNDS: Range<f32> = -17.0..17.0;
const Z_BOUNDS: Range<f32> = -50.0..-30.0;
const RADIUS: Range<f32> = 0.5..2.0;
const SPIN: Range<f32> = -0.5..0.5;

const RESOLUTION: u32 = 1000;

/// how long does a gizmo "blink" off, in seconds
const OFF_TIME: Range<f32> = 0.1..0.3;
/// delay between twinkles, in seconds
const TWINKLE_DELAY: Range<f32> = 5.0..10.0;

#[derive(Component)]
struct Spin(f32);

#[derive(Component)]
struct Twinkler {
    timer: Timer,
    not_before: f32,
}

#[derive(Deref, DerefMut)]
struct SpawnTimer(Timer);

impl Default for SpawnTimer {
    fn default() -> Self {
        SpawnTimer(Timer::from_seconds(0.1, TimerMode::Once))
    }
}

fn spawn_gizmo(
    mut commands: Commands,
    mut timer: Local<SpawnTimer>,
    gizmos: Query<Entity, With<Gizmo>>,
    time: Res<Time>,
) {
    timer.tick(time.delta());
    if !timer.just_finished() {
        return;
    }
    let mut rng = rand::rng();
    **timer = Timer::from_seconds(rng.random_range(DELAY_RANGE), TimerMode::Once);

    let total = gizmos.iter().count();

    if total > MIN_COUNT && rng.random_bool((total as f64 / SOFT_PEAK).min(1.0)) {
        if let Some(entity) = gizmos.iter().nth(rng.random_range(0..total)) {
            commands.entity(entity).despawn();
        }
        return;
    }

    commands.trigger(SpawnGizmo);
}

#[derive(Event)]
struct SpawnGizmo;

fn on_spawn_gizmo(
    _: On<SpawnGizmo>,
    mut commands: Commands,
    mut gizmo_assets: ResMut<Assets<GizmoAsset>>,
) {
    let mut gizmo = GizmoAsset::new();

    let mut rng = rand::rng();
    let radius = rng.random_range(RADIUS);
    let x = rng.random_range(X_BOUNDS);
    let y = rng.random_range(Y_BOUNDS);
    let z = rng.random_range(Z_BOUNDS);
    // randomize an orientation
    let axis = Vec3::new(
        rng.random_range(-1.0..1.0),
        rng.random_range(-1.0..1.0),
        rng.random_range(-1.0..1.0),
    )
    .normalize_or(Vec3::Y);
    let up = axis.any_orthonormal_vector();
    let spin_rate = rng.random_range(SPIN);

    // When drawing a lot of static lines a Gizmo component can have
    // far better performance than the Gizmos system parameter,
    // but the system parameter will perform better for smaller lines that update often.
    let color = COLORS[rng.random_range(0..COLORS.len())];
    gizmo
        .sphere(Isometry3d::IDENTITY, radius, color)
        .resolution(RESOLUTION);

    let color = COLORS[rng.random_range(0..COLORS.len())];
    gizmo.cross(Isometry3d::IDENTITY, radius, color);

    commands.spawn((
        Gizmo {
            handle: gizmo_assets.add(gizmo),
            line_config: GizmoLineConfig {
                width: 2.,
                ..default()
            },
            ..default()
        },
        Transform::from_translation(vec3(x, y, z)).looking_to(axis, up),
        Spin(spin_rate),
    ));
}

fn spin(mut spinners: Query<(&mut Transform, &Spin)>, time: Res<Time>) {
    for (mut transform, spin) in &mut spinners {
        transform.rotate_local_y(time.delta_secs() * spin.0);
    }
}

fn twinkle(
    mut commands: Commands,
    mut gizmos: Query<(Entity, &mut Visibility, Option<&mut Twinkler>), With<Gizmo>>,
    time: Res<Time>,
) {
    for (entity, mut visibility, twinkler) in &mut gizmos {
        if let Some(mut twinkler) = twinkler {
            twinkler.timer.tick(time.delta());
            if twinkler.timer.just_finished() {
                *visibility = Visibility::Visible;
            }
            if twinkler.not_before < time.elapsed_secs() {
                commands.entity(entity).remove::<Twinkler>();
            }
        } else if rand::rng().random_bool(0.001) {
            *visibility = Visibility::Hidden;
            commands.entity(entity).insert(Twinkler {
                timer: Timer::from_seconds(rand::rng().random_range(OFF_TIME), TimerMode::Once),
                not_before: time.elapsed_secs() + rand::rng().random_range(TWINKLE_DELAY),
            });
        }
    }
}

fn setup(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 0., 6.).looking_at(Vec3::NEG_Z, Vec3::Y),
        FreeCamera::default(),
    ));
}

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, FreeCameraPlugin))
        .insert_resource(ClearColor(BLACK.into()))
        .add_systems(Startup, setup)
        .add_systems(Update, (spawn_gizmo, spin, twinkle))
        .add_observer(on_spawn_gizmo)
        .run();
}
