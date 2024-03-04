//! This example demonstrates the implementation and behavior of the axes gizmo.
use bevy::prelude::*;
use rand::random;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, draw_axes)
        .run();
}

#[derive(Component)]
struct ShowAxes {
    base_length: f32,
}

#[derive(Component)]
struct TransformTracking {
    initial_transform: Transform,
    target_transform: Transform,
    progress: u16,
}

fn setup(mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Lights...
    commands.spawn(
        PointLightBundle {
            point_light: PointLight {
                shadows_enabled: true,
                ..default()
            },
            transform: Transform::from_xyz(2., 6., 0.),
            ..default()
        }
    );

    // Camera...
    commands.spawn(Camera3dBundle {
        transform: Transform::from_xyz(0., 1.5, 8.).looking_at(Vec3::new(0., -0.5, 0.), Vec3::Y),
        ..default()
    });

    // Action! (Our cube that's going to move)
    commands.spawn((
        PbrBundle {
            mesh: meshes.add(Cuboid::new(1., 1., 1.)),
            material: materials.add(Color::srgb(0.8, 0.7, 0.6)),
            ..default()
        },
        ShowAxes {
            base_length: 1.5,
        },
        TransformTracking {
            initial_transform: default(),
            target_transform: random_transform(),
            progress:0,
        }
    ));

    // A plane to give a sense of place
    commands.spawn(
        PbrBundle {
            mesh: meshes.add(Plane3d::default().mesh().size(20., 20.)),
            material: materials.add(Color::srgb(0.1, 0.1, 0.1)),
            transform: Transform::from_xyz(0., -2., 0.),
            ..default()
        }
    );
}

// Draw the axes based on the cube's transform.
fn draw_axes(
    mut gizmos: Gizmos,
    query: Query<(&Transform, &ShowAxes)>
) {
    let (&transform, axis_data) = query.single();
    gizmos.axes(transform, axis_data.base_length);
}

// Helper functions:

const TRANSLATION_BOUND_LOWER_X: f32 = -18.;
const TRANSLATION_BOUND_UPPER_X: f32 = 18.;
const TRANSLATION_BOUND_LOWER_Y: f32 = -1.;
const TRANSLATION_BOUND_UPPER_Y: f32 = 5.;
const TRANSLATION_BOUND_LOWER_Z: f32 = -3.;
const TRANSLATION_BOUND_UPPER_Z: f32 = 18.;

const SCALING_BOUND_LOWER_LOG: f32 = -1.;
const SCALING_BOUND_UPPER_LOG: f32 = 1.;

fn random_transform() -> Transform {
    default()
}

fn random_translation() -> Vec3 {
    let x = random::<f32>() * (TRANSLATION_BOUND_UPPER_X - TRANSLATION_BOUND_LOWER_X) + TRANSLATION_BOUND_LOWER_X;
    let y = random::<f32>() * (TRANSLATION_BOUND_UPPER_Y - TRANSLATION_BOUND_LOWER_Y) + TRANSLATION_BOUND_LOWER_Y;
    let z = random::<f32>() * (TRANSLATION_BOUND_UPPER_Z - TRANSLATION_BOUND_LOWER_Z) + TRANSLATION_BOUND_LOWER_Z;

    Vec3::new(x, y, z)
}

fn random_scale() -> Vec3 {
    let x_factor_log = random::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG) + SCALING_BOUND_LOWER_LOG;
    let y_factor_log = random::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG) + SCALING_BOUND_LOWER_LOG;
    let z_factor_log = random::<f32>() * (SCALING_BOUND_UPPER_LOG - SCALING_BOUND_LOWER_LOG) + SCALING_BOUND_LOWER_LOG;

    let x_factor = x_factor_log.exp2();
    let y_factor = y_factor_log.exp2();
    let z_factor = z_factor_log.exp2();

    Vec3::new(
        x_factor_log.exp2(),
        y_factor_log.exp2(), 
        z_factor_log.exp2()
    )
}

fn random_rotation() -> Quat {
    default() // TODO
}

fn elerp(v1: Vec3, v2: Vec3, t: f32) -> Vec3 {
    let x_factor_log = (1. - t) * v1.x.log2() + t * v1.x.log2();
    let y_factor_log = (1. - t) * v1.y.log2() + t * v1.y.log2();
    let z_factor_log = (1. - t) * v1.z.log2() + t * v1.z.log2();

    Vec3::new(
        x_factor_log.exp2(),
        y_factor_log.exp2(), 
        z_factor_log.exp2()
    )
}