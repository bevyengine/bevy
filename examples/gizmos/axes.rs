//! This example demonstrates the implementation and behavior of the axes gizmo.
use bevy::prelude::*;
use bevy::render::primitives::Aabb;
use rand::random;
use std::f32::consts::PI;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_cubes, draw_axes))
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

    /// The progress of the cube during the move in percentage points
    progress: u16,
}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Lights...
    commands.spawn(PointLightBundle {
        point_light: PointLight {
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(2., 6., 0.),
        ..default()
    });

    // Camera...
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0., 1.5, -8.).looking_at(Vec3::new(0., -0.5, 0.), Vec3::Y),
        ..default()
    });

    // Action! (Our cubes that are going to move)
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1., 1., 1.)),
            material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
            ..default()
        },
        ShowAxes,
        TransformTracking {
            initial_transform: default(),
            target_transform: random_transform(),
            progress: 0,
        },
    ));

    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(0.5, 0.5, 0.5)),
            material: materials.add(Color::srgb(0.6, 0.7, 0.8)),
            ..default()
        },
        ShowAxes,
        TransformTracking {
            initial_transform: default(),
            target_transform: random_transform(),
            progress: 0,
        },
    ));

    // A plane to give a sense of place
    commands.spawn(PbrBundle {
        mesh: meshes.add(Plane3d::default().mesh().size(20., 20.)),
        material: materials.add(Color::srgb(0.1, 0.1, 0.1)),
        transform: Transform::from_xyz(0., -2., 0.),
        ..default()
    });
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
fn move_cubes(mut query: Query<(&mut Transform, &mut TransformTracking)>) {
    for (mut transform, mut tracking) in &mut query {
        let t = tracking.progress as f32 / 100.;

        *transform =
            interpolate_transforms(tracking.initial_transform, tracking.target_transform, t);

        if tracking.progress < 100 {
            tracking.progress += 1;
        } else {
            tracking.initial_transform = *transform;
            tracking.target_transform = random_transform();
            tracking.progress = 0;
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

fn random_transform() -> Transform {
    Transform {
        translation: random_translation(),
        rotation: random_rotation(),
        scale: random_scale(),
    }
}

fn random_translation() -> Vec3 {
    let x = random::<f32>() * (TRANSLATION_BOUND_UPPER_X - TRANSLATION_BOUND_LOWER_X)
        + TRANSLATION_BOUND_LOWER_X;
    let y = random::<f32>() * (TRANSLATION_BOUND_UPPER_Y - TRANSLATION_BOUND_LOWER_Y)
        + TRANSLATION_BOUND_LOWER_Y;
    let z = random::<f32>() * (TRANSLATION_BOUND_UPPER_Z - TRANSLATION_BOUND_LOWER_Z)
        + TRANSLATION_BOUND_LOWER_Z;

    Vec3::new(x, y, z)
}

fn random_scale() -> Vec3 {
    let x_factor_log = random::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG)
        + SCALING_BOUND_LOWER_LOG;
    let y_factor_log = random::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG)
        + SCALING_BOUND_LOWER_LOG;
    let z_factor_log = random::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG)
        + SCALING_BOUND_LOWER_LOG;

    Vec3::new(
        x_factor_log.exp2(),
        y_factor_log.exp2(),
        z_factor_log.exp2(),
    )
}

fn elerp(v1: Vec3, v2: Vec3, t: f32) -> Vec3 {
    let x_factor_log = (1. - t) * v1.x.log2() + t * v2.x.log2();
    let y_factor_log = (1. - t) * v1.y.log2() + t * v2.y.log2();
    let z_factor_log = (1. - t) * v1.z.log2() + t * v2.z.log2();

    Vec3::new(
        x_factor_log.exp2(),
        y_factor_log.exp2(),
        z_factor_log.exp2(),
    )
}

fn random_rotation() -> Quat {
    let dir = random_direction();
    let angle = random::<f32>() * 2. * PI;

    Quat::from_axis_angle(dir, angle)
}

fn random_direction() -> Vec3 {
    let height = random::<f32>() * 2. - 1.;
    let theta = random::<f32>() * 2. * PI;

    build_direction(height, theta)
}

fn build_direction(height: f32, theta: f32) -> Vec3 {
    let z = height;
    let m = f32::acos(z).sin();
    let x = theta.cos() * m;
    let y = theta.sin() * m;

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
