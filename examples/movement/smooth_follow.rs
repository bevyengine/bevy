//! This example demonstrates how to use interpolation to make one entity smoothly follow another.

use bevy::{
    math::{prelude::*, vec3, NormedVectorSpace},
    prelude::*,
};
use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_target, move_follower).chain())
        .run();
}

// The sphere that the following sphere targets at all times:
#[derive(Component)]
struct TargetSphere;

// The speed of the target sphere moving to its next location:
#[derive(Resource)]
struct TargetSphereSpeed(f32);

// The position that the target sphere always moves linearly toward:
#[derive(Resource)]
struct TargetPosition(Vec3);

// The decay rate used by the smooth following:
#[derive(Resource)]
struct DecayRate(f32);

// The sphere that follows the target sphere by moving towards it with nudging:
#[derive(Component)]
struct FollowingSphere;

/// The source of randomness used by this example.
#[derive(Resource)]
struct RandomSource(ChaCha8Rng);

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // A plane:
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::default().mesh().size(12.0, 12.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.15, 0.3))),
        Transform::from_xyz(0.0, -2.5, 0.0),
    ));

    // The target sphere:
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.3))),
        MeshMaterial3d(materials.add(Color::srgb(0.3, 0.15, 0.9))),
        TargetSphere,
    ));

    // The sphere that follows it:
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(0.3))),
        MeshMaterial3d(materials.add(Color::srgb(0.9, 0.3, 0.3))),
        Transform::from_translation(vec3(0.0, -2.0, 0.0)),
        FollowingSphere,
    ));

    // A light:
    commands.spawn((
        PointLight {
            intensity: 15_000_000.0,
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));

    // A camera:
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.0, 3.0, 5.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));

    // Set starting values for resources used by the systems:
    commands.insert_resource(TargetSphereSpeed(5.0));
    commands.insert_resource(DecayRate(2.0));
    commands.insert_resource(TargetPosition(Vec3::ZERO));
    commands.insert_resource(RandomSource(ChaCha8Rng::seed_from_u64(68941654987813521)));
}

fn move_target(
    mut target: Single<&mut Transform, With<TargetSphere>>,
    target_speed: Res<TargetSphereSpeed>,
    mut target_pos: ResMut<TargetPosition>,
    time: Res<Time>,
    mut rng: ResMut<RandomSource>,
) {
    match Dir3::new(target_pos.0 - target.translation) {
        // The target and the present position of the target sphere are far enough to have a well-
        // defined direction between them, so let's move closer:
        Ok(dir) => {
            let delta_time = time.delta_secs();
            let abs_delta = (target_pos.0 - target.translation).norm();

            // Avoid overshooting in case of high values of `delta_time`:
            let magnitude = f32::min(abs_delta, delta_time * target_speed.0);
            target.translation += dir * magnitude;
        }

        // The two are really close, so let's generate a new target position:
        Err(_) => {
            let legal_region = Cuboid::from_size(Vec3::splat(4.0));
            *target_pos = TargetPosition(legal_region.sample_interior(&mut rng.0));
        }
    }
}

fn move_follower(
    mut following: Single<&mut Transform, With<FollowingSphere>>,
    target: Single<&Transform, (With<TargetSphere>, Without<FollowingSphere>)>,
    decay_rate: Res<DecayRate>,
    time: Res<Time>,
) {
    let decay_rate = decay_rate.0;
    let delta_time = time.delta_secs();

    // Calling `smooth_nudge` is what moves the following sphere smoothly toward the target.
    following
        .translation
        .smooth_nudge(&target.translation, decay_rate, delta_time);
}
