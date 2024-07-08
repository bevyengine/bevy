//! This example shows how to properly handle player input,
//! advance a physics simulation in a fixed timestep, and display the results.
//!
//! The classic source for how and why this is done is Glenn Fiedler's article
//! [Fix Your Timestep!](https://gafferongames.com/post/fix_your_timestep/).
//!
//! ## Motivation
//!
//! The naive way of moving a player is to just update their position like so:
//! ```no_run
//! transform.translation += velocity;
//! ```
//! The issue here is that the player's movement speed will be tied to the frame rate.
//! Faster machines will move the player faster, and slower machines will move the player slower.
//! In fact, you can observe this today when running some old games that did it this way on modern hardware!
//! The player will move at a breakneck pace.
//!
//! The more sophisticated way is to update the player's position based on the time that has passed:
//! ```no_run
//! transform.translation += velocity * time.delta_seconds();
//! ```
//! This way, velocity represents a speed in units per second, and the player will move at the same speed
//! regardless of the frame rate.
//!
//! However, this can still be problematic if the frame rate is very low or very high.
//! If the frame rate is very low, the player will move in large jumps. This may lead to
//! a player moving in such large jumps that they pass through walls or other obstacles.
//! In general, you cannot expect a physics simulation to behave nicely with *any* delta time.
//! Ideally, we want to have some stability in what kinds of delta times we feed into our physics simulation.
//!
//! The solution is using a fixed timestep. This means that we advance the physics simulation by a fixed amount
//! at a time. If the real time that passed between two frames is less than the fixed timestep, we simply
//! don't advance the physics simulation at all.
//! If it is more, we advance the physics simulation multiple times until we catch up.
//! You can read more about how Bevy implements this in the documentation for
//! [`bevy::time::Fixed`](https://docs.rs/bevy/latest/bevy/time/struct.Fixed.html).
//!
//! This leaves us with a last problem, however. If our physics simulation may advance zero or multiple times
//! per frame, there may be frames in which the player's position did not need to be updated at all,
//! and some where it is updated by a large amount that resulted from running the physics simulation multiple times.
//! This is physically correct, but visually jarring. Imagine a player moving in a straight line, but depending on the frame rate,
//! they may sometimes advance by a large amount and sometimes not at all. Visually, we want the player to move smoothly.
//! This is why we need to separate the player's position in the physics simulation from the player's position in the visual representation.
//! The visual representation can then be interpolated smoothly based on the last displayed position and
//! the player's actual position in the physics simulation.
//!
//! There are other ways to handle the visual representation of the player, such as extrapolation.
//! See the [documentation of the lightyear crate](https://cbournhonesque.github.io/lightyear/book/concepts/advanced_replication/visual_interpolation.html)
//! for a nice overview of the different methods.
//!
//! ## Implementation
//!
//! - The player's velocity is stored in a `Velocity` component. This is the speed in units per second.
//! - The player's position in the physics simulation is stored in a `PhysicalTranslation` component.
//! - The player's visual representation is stored in Bevy's regular `Transform` component.
//! - Every frame, we go through the following steps:
//!    - Advance the physics simulation by one fixed timestep in the `advance_physics` system.
//!        This is run in the `FixedUpdate` schedule, which runs before the `Update` schedule.
//!    - Update the player's visual representation in the `update_displayed_transform` system.
//!        This interpolates between the last displayed position and the actual position in the physics simulation.
//!    - Update the player's velocity based on the player's input in the `handle_input` system.
//!
//!
//! ## Controls
//!
//! | Key Binding          | Action        |
//! |:---------------------|:--------------|
//! | `W`                  | Move up       |
//! | `S`                  | Move down     |
//! | `A`                  | Move left     |
//! | `D`                  | Move right    |

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, (spawn_text, spawn_player))
        // `FixedUpdate` runs before `Update`, so the physics simulation is advanced before the player's visual representation is updated.
        .add_systems(FixedUpdate, advance_physics)
        .add_systems(Update, (update_displayed_transform, handle_input).chain())
        .run();
}

/// How many units per second the player should move.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct Velocity(Vec2);

/// The actual position of the player in the physics simulation.
/// This is separate from the `Transform`, which is merely a visual representation.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct PhysicalTranslation(Vec2);

/// Spawn the player sprite and a 2D camera.
fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    commands.spawn((
        Name::new("Player"),
        SpriteBundle {
            texture: asset_server.load("branding/icon.png"),
            transform: Transform::from_xyz(100., 0., 0.).with_scale(Vec3::splat(0.3)),
            ..default()
        },
        Velocity::default(),
        PhysicalTranslation::default(),
    ));
}

/// Spawn a bit of UI text to explain how to move the player.
fn spawn_text(mut commands: Commands) {
    commands
        .spawn(NodeBundle {
            style: Style {
                position_type: PositionType::Absolute,
                bottom: Val::Px(12.0),
                left: Val::Px(12.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            parent.spawn(TextBundle::from_section(
                "Move the player with WASD",
                TextStyle {
                    font_size: 25.0,
                    ..default()
                },
            ));
        });
}

/// Handle keyboard input to move the player.
fn handle_input(keyboard_input: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Velocity>) {
    /// Since Bevy's default 2D camera setup is scaled such that
    /// one unit is one pixel, you can think of this as
    /// "How many pixels per second should the player move?"
    const SPEED: f32 = 210.0;
    for mut velocity in query.iter_mut() {
        velocity.0 = Vec2::ZERO;

        if keyboard_input.pressed(KeyCode::KeyW) {
            velocity.y += SPEED;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            velocity.y -= SPEED;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            velocity.x -= SPEED;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            velocity.x += SPEED;
        }

        // Need to normalize and scale because otherwise
        // diagonal movement would be faster than horizontal or vertical movement.
        velocity.0 = velocity.normalize_or_zero() * SPEED;
    }
}

/// Advance the physics simulation by one fixed timestep. This may run zero or multiple times per frame.
///
/// Note that since this runs in `FixedUpdate`, `Res<Time>` would be `Res<Time<Fixed>>` automatically.
/// We are being explicit here for clarity.
fn advance_physics(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(&mut PhysicalTranslation, &Velocity)>,
) {
    for (mut physical_translation, velocity) in query.iter_mut() {
        physical_translation.0 += velocity.0 * fixed_time.delta_seconds();
    }
}

fn update_displayed_transform(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(&mut Transform, &PhysicalTranslation)>,
) {
    for (mut transform, physical_translation) in query.iter_mut() {
        let last_displayed_translation = transform.translation;
        let actual_translation = physical_translation.0.extend(0.0);
        // The overstep fraction is a value between 0 and 1 that tells us how far we are between two fixed timesteps.
        let alpha = fixed_time.overstep_fraction();

        let next_displayed_translation = last_displayed_translation.lerp(actual_translation, alpha);

        transform.translation = next_displayed_translation;
    }
}
