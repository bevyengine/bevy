//! This example shows how to sample random points from primitive shapes.

use bevy::math::prelude::*;
use bevy::prelude::*;
use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        // .add_systems(Update, )
        .run();
}

/// Marker component for the sphere used by
#[derive(Resource)]
struct TheSphere {}

/// The source of randomness used by this example.
#[derive(Resource)]
struct RandomSource(ChaCha8Rng);

/// A container for the handle storing the mesh used to display sampled points as spheres.
#[derive(Resource)]
struct PointMesh(Handle<Mesh>);

/// A container for the handle storing the material used to display sampled points.
#[derive(Resource)]
struct PointMaterial(Handle<StandardMaterial>);

/// Marker component for sampled points.
#[derive(Component)]
struct SampledPoint {}

fn setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Use seeded rng and store it in a resource; this makes the random output reproducible.
    let seeded_rng = ChaCha8Rng::seed_from_u64(19878367467712);
    commands.insert_resource(RandomSource(seeded_rng));
}
