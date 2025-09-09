//! This example demonstrates the implementation and behavior of the axes gizmo.

use bevy::{camera::primitives::Aabb, prelude::*};
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use std::f32::consts::PI;

const TRANSITION_DURATION: f32 = 2.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_cubes, draw_axes).chain())
        .run();
}

/// The `ShowAxes` component is attached to an entity to get the `draw_axes` system to
/// display axes according to its Transform component.
#[derive(Component)]
struct ShowAxes;

/// The `TransformTracking` component keeps track of the data we need to interpolate
/// between two transforms in our example.
#[derive(Component)]
struct TransformTracking {
    /// The initial transform of the cube during the move
    initial_transform: Transform,

    /// The target transform of the cube during the move
    target_transform: Transform,

    /// The progress of the cube during the move in seconds
    progress: f32,
}

#[derive(Resource)]
struct SeededRng(ChaCha8Rng);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // We're seeding the PRNG here to make this example deterministic for testing purposes.
    // This isn't strictly required in practical use unless you need your app to be deterministic.
    let mut rng = ChaCha8Rng::seed_from_u64(19878367467713);

    // Lights...
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(2., 6., 0.),
    ));

    // Camera...
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(0., 1.5, -8.).looking_at(Vec3::new(0., -0.5, 0.), Vec3::Y),
    ));

    // Action! (Our cubes that are going to move)
    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1., 1., 1.))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.7, 0.6))),
        ShowAxes,
        TransformTracking {
            initial_transform: default(),
            target_transform: random_transform(&mut rng),
            progress: 0.0,
        },
    ));

    commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 0.5))),
        MeshMaterial3d(materials.add(Color::srgb(0.6, 0.7, 0.8))),
        ShowAxes,
        TransformTracking {
            initial_transform: default(),
            target_transform: random_transform(&mut rng),
            progress: 0.0,
        },
    ));

    // A plane to give a sense of place
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(20., 20.))),
        MeshMaterial3d(materials.add(Color::srgb(0.1, 0.1, 0.1))),
        Transform::from_xyz(0., -2., 0.),
    ));

    commands.insert_resource(SeededRng(rng));
}

// This system draws the axes based on the cube's transform, with length based on the size of
// the entity's axis-aligned bounding box (AABB).
fn draw_axes(mut gizmos: Gizmos, query: Query<(&Transform, &Aabb), With<ShowAxes>>) {
    for (&transform, &aabb) in &query {
        let length = aabb.half_extents.length();
        gizmos.axes(transform, length);
    }
}

// This system changes the cubes' transforms to interpolate between random transforms
fn move_cubes(
    mut query: Query<(&mut Transform, &mut TransformTracking)>,
    time: Res<Time>,
    mut rng: ResMut<SeededRng>,
) {
    for (mut transform, mut tracking) in &mut query {
        *transform = interpolate_transforms(
            tracking.initial_transform,
            tracking.target_transform,
            tracking.progress / TRANSITION_DURATION,
        );

        if tracking.progress < TRANSITION_DURATION {
            tracking.progress += time.delta_secs();
        } else {
            tracking.initial_transform = *transform;
            tracking.target_transform = random_transform(&mut rng.0);
            tracking.progress = 0.0;
        }
    }
}

// Helper functions for random transforms and interpolation:

const TRANSLATION_BOUND_LOWER_X: f32 = -5.;
const TRANSLATION_BOUND_UPPER_X: f32 = 5.;
const TRANSLATION_BOUND_LOWER_Y: f32 = -1.;
const TRANSLATION_BOUND_UPPER_Y: f32 = 1.;
const TRANSLATION_BOUND_LOWER_Z: f32 = -2.;
const TRANSLATION_BOUND_UPPER_Z: f32 = 6.;

const SCALING_BOUND_LOWER_LOG: f32 = -1.2;
const SCALING_BOUND_UPPER_LOG: f32 = 1.2;

fn random_transform(rng: &mut impl Rng) -> Transform {
    Transform {
        translation: random_translation(rng),
        rotation: random_rotation(rng),
        scale: random_scale(rng),
    }
}

fn random_translation(rng: &mut impl Rng) -> Vec3 {
    let x = rng.random::<f32>() * (TRANSLATION_BOUND_UPPER_X - TRANSLATION_BOUND_LOWER_X)
        + TRANSLATION_BOUND_LOWER_X;
    let y = rng.random::<f32>() * (TRANSLATION_BOUND_UPPER_Y - TRANSLATION_BOUND_LOWER_Y)
        + TRANSLATION_BOUND_LOWER_Y;
    let z = rng.random::<f32>() * (TRANSLATION_BOUND_UPPER_Z - TRANSLATION_BOUND_LOWER_Z)
        + TRANSLATION_BOUND_LOWER_Z;

    Vec3::new(x, y, z)
}

fn random_scale(rng: &mut impl Rng) -> Vec3 {
    let x_factor_log = rng.random::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG)
        + SCALING_BOUND_LOWER_LOG;
    let y_factor_log = rng.random::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG)
        + SCALING_BOUND_LOWER_LOG;
    let z_factor_log = rng.random::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG)
        + SCALING_BOUND_LOWER_LOG;

    Vec3::new(
        ops::exp2(x_factor_log),
        ops::exp2(y_factor_log),
        ops::exp2(z_factor_log),
    )
}

fn elerp(v1: Vec3, v2: Vec3, t: f32) -> Vec3 {
    let x_factor_log = (1. - t) * ops::log2(v1.x) + t * ops::log2(v2.x);
    let y_factor_log = (1. - t) * ops::log2(v1.y) + t * ops::log2(v2.y);
    let z_factor_log = (1. - t) * ops::log2(v1.z) + t * ops::log2(v2.z);

    Vec3::new(
        ops::exp2(x_factor_log),
        ops::exp2(y_factor_log),
        ops::exp2(z_factor_log),
    )
}

fn random_rotation(rng: &mut impl Rng) -> Quat {
    let dir = random_direction(rng);
    let angle = rng.random::<f32>() * 2. * PI;

    Quat::from_axis_angle(dir, angle)
}

fn random_direction(rng: &mut impl Rng) -> Vec3 {
    let height = rng.random::<f32>() * 2. - 1.;
    let theta = rng.random::<f32>() * 2. * PI;

    build_direction(height, theta)
}

fn build_direction(height: f32, theta: f32) -> Vec3 {
    let z = height;
    let m = ops::sin(ops::acos(z));
    let x = ops::cos(theta) * m;
    let y = ops::sin(theta) * m;

    Vec3::new(x, y, z)
}

fn interpolate_transforms(t1: Transform, t2: Transform, t: f32) -> Transform {
    let translation = t1.translation.lerp(t2.translation, t);
    let rotation = t1.rotation.slerp(t2.rotation, t);
    let scale = elerp(t1.scale, t2.scale, t);

    Transform {
        translation,
        rotation,
        scale,
    }
}
