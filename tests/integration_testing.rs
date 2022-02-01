//! Integration testing Bevy apps is surprisingly easy,
//! and is a great tool for ironing out tricky bugs or enabling refactors.
//!
//! Create new files in your root `tests` directory, and then call `cargo test` to ensure that they pass.
//!
//! You can easily reuse functionality between your tests and game by organizing your logic with plugins,
//! and then use direct methods on `App` / `World` to set up test scenarios.

use bevy::prelude::*;

// This plugin should be defined in your `src` folder, and exported from your project
pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        app.add_startup_system(spawn_player)
            .add_system(jump)
            .add_system(gravity)
            .add_system(apply_velocity)
            .add_system_to_stage(CoreStage::PostUpdate, clamp_position);
    }
}

#[derive(Component)]
struct Player;

#[derive(Component, Default)]
struct Velocity(Vec3);

// These systems don't need to be `pub`, as they're hidden within your plugin
fn spawn_player(mut commands: Commands) {
    commands
        .spawn()
        .insert(Player)
        .insert(Transform::default())
        .insert(Velocity::default());
}

fn apply_velocity(query: Query<(&mut Transform, &Velocity)>) {}

fn jump() {}

fn gravity() {}

/// Players should not fall through the floor
fn clamp_position() {}
