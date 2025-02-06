//! Generating a collection of "prefab" entities can be faster and cleaner than
//! loading them from assets each time or working entirely in code.
//!
//! Rather than providing an opinonated prefab system, Bevy provides a flexible
//! set of tools that can be used to create and modify your solution.
//!
//! The core workflow is pretty straightforward:
//!
//! 1. Load asssets from disk.
//! 2. Create prefab entities from those assets.
//! 3. Make sure that these prefab entities aren't accidentally modified using default query filters.
//! 4. Clone these entities (and their children) out from the prefab when you need to spawn an instance of them.
//!
//! This solution can be easily adapted to meet the needs of your own asset loading workflows,
//! and variants of prefabs (e.g. enemy variants) can readily be constructed ahead of time and stored for easy access.
//!
//! Be mindful of memory usage when defining prefabs; while they won't be seen by game logic,
//! the components and assets that they use will still be loaded into memory (although asset data is shared between instances).
//! Loading and unloading assets dynamically (e.g. per level) is an important strategy to manage memory usage.

use bevy::prelude::*;

#[derive(States, Debug, PartialEq, Eq, Clone, Copy, Hash)]
enum AssetLoadingState {
    Loading,
    Loaded,
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup_scene)
        .add_systems(OnEnter(AssetLoadingState::Loading), load_models)
        .run();
}

fn setup_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    // Circular floor to display our models on
    commands.spawn((
        Mesh3d(meshes.add(Circle::new(4.0))),
        MeshMaterial3d(materials.add(Color::WHITE)),
        Transform::from_rotation(Quat::from_rotation_x(-std::f32::consts::FRAC_PI_2)),
    ));
    // Light
    commands.spawn((
        PointLight {
            shadows_enabled: true,
            ..default()
        },
        Transform::from_xyz(4.0, 8.0, 4.0),
    ));
    // Camera
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(-2.5, 4.5, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
    ));
}

fn load_models() {}
