//! This example shows how to properly handle player input,
//! advance a physics simulation in a fixed timestep, and display the results.
//!
//! The classic source for how and why this is done is Glenn Fiedler's article
//! [Fix Your Timestep!](https://gafferongames.com/post/fix_your_timestep/).
//! For a more Bevy-centric source, see
//! [this cheatbook entry](https://bevy-cheatbook.github.io/fundamentals/fixed-timestep.html).
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
//! transform.translation += velocity * time.delta_secs();
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
//! The visual representation can then be interpolated smoothly based on the previous and current actual player position in the physics simulation.
//!
//! This is a tradeoff: every visual frame is now slightly lagging behind the actual physical frame,
//! but in return, the player's movement will appear smooth.
//! There are other ways to compute the visual representation of the player, such as extrapolation.
//! See the [documentation of the lightyear crate](https://cbournhonesque.github.io/lightyear/book/concepts/advanced_replication/visual_interpolation.html)
//! for a nice overview of the different methods and their respective tradeoffs.
//!
//! ## Implementation
//!
//! - The player's inputs since the last physics update are stored in the `AccumulatedInput` component.
//! - The player's velocity is stored in a `Velocity` component. This is the speed in units per second.
//! - The player's current position in the physics simulation is stored in a `PhysicalTranslation` component.
//! - The player's previous position in the physics simulation is stored in a `PreviousPhysicalTranslation` component.
//! - The player's visual representation is stored in Bevy's regular `Transform` component.
//! - Every frame, we go through the following steps:
//!   - Accumulate the player's input and set the current speed in the `handle_input` system.
//!     This is run in the `RunFixedMainLoop` schedule, ordered in `RunFixedMainLoopSystem::BeforeFixedMainLoop`,
//!     which runs before the fixed timestep loop. This is run every frame.
//!   - Advance the physics simulation by one fixed timestep in the `advance_physics` system.
//!     Accumulated input is consumed here.
//!     This is run in the `FixedUpdate` schedule, which runs zero or multiple times per frame.
//!   - Update the player's visual representation in the `interpolate_rendered_transform` system.
//!     This interpolates between the player's previous and current position in the physics simulation.
//!     It is run in the `RunFixedMainLoop` schedule, ordered in `RunFixedMainLoopSystem::AfterFixedMainLoop`,
//!     which runs after the fixed timestep loop. This is run every frame.
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
        // Advance the physics simulation using a fixed timestep.
        .add_systems(FixedUpdate, advance_physics)
        .add_systems(
            // The `RunFixedMainLoop` schedule allows us to schedule systems to run before and after the fixed timestep loop.
            RunFixedMainLoop,
            (
                // The physics simulation needs to know the player's input, so we run this before the fixed timestep loop.
                // Note that if we ran it in `Update`, it would be too late, as the physics simulation would already have been advanced.
                // If we ran this in `FixedUpdate`, it would sometimes not register player input, as that schedule may run zero times per frame.
                handle_input.in_set(RunFixedMainLoopSystem::BeforeFixedMainLoop),
                // The player's visual representation needs to be updated after the physics simulation has been advanced.
                // This could be run in `Update`, but if we run it here instead, the systems in `Update`
                // will be working with the `Transform` that will actually be shown on screen.
                interpolate_rendered_transform.in_set(RunFixedMainLoopSystem::AfterFixedMainLoop),
            ),
        )
        .run();
}

/// A vector representing the player's input, accumulated over all frames that ran
/// since the last time the physics simulation was advanced.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct AccumulatedInput(Vec2);

/// A vector representing the player's velocity in the physics simulation.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct Velocity(Vec3);

/// The actual position of the player in the physics simulation.
/// This is separate from the `Transform`, which is merely a visual representation.
///
/// If you want to make sure that this component is always initialized
/// with the same value as the `Transform`'s translation, you can
/// use a [component lifecycle hook](https://docs.rs/bevy/0.14.0/bevy/ecs/component/struct.ComponentHooks.html)
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct PhysicalTranslation(Vec3);

/// The value [`PhysicalTranslation`] had in the last fixed timestep.
/// Used for interpolation in the `interpolate_rendered_transform` system.
#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct PreviousPhysicalTranslation(Vec3);

/// Spawn the player sprite and a 2D camera.
fn spawn_player(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2d);
    commands.spawn((
        Name::new("Player"),
        Sprite::from_image(asset_server.load("branding/icon.png")),
        Transform::from_scale(Vec3::splat(0.3)),
        AccumulatedInput::default(),
        Velocity::default(),
        PhysicalTranslation::default(),
        PreviousPhysicalTranslation::default(),
    ));
}

/// Spawn a bit of UI text to explain how to move the player.
fn spawn_text(mut commands: Commands) {
    commands
        .spawn(Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        })
        .with_child((
            Text::new("Move the player with WASD"),
            TextFont {
                font_size: 25.0,
                ..default()
            },
        ));
}

/// Handle keyboard input and accumulate it in the `AccumulatedInput` component.
///
/// There are many strategies for how to handle all the input that happened since the last fixed timestep.
/// This is a very simple one: we just accumulate the input and average it out by normalizing it.
fn handle_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut query: Query<(&mut AccumulatedInput, &mut Velocity)>,
) {
    /// Since Bevy's default 2D camera setup is scaled such that
    /// one unit is one pixel, you can think of this as
    /// "How many pixels per second should the player move?"
    const SPEED: f32 = 210.0;
    for (mut input, mut velocity) in query.iter_mut() {
        if keyboard_input.pressed(KeyCode::KeyW) {
            input.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyS) {
            input.y -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyA) {
            input.x -= 1.0;
        }
        if keyboard_input.pressed(KeyCode::KeyD) {
            input.x += 1.0;
        }

        // Need to normalize and scale because otherwise
        // diagonal movement would be faster than horizontal or vertical movement.
        // This effectively averages the accumulated input.
        velocity.0 = input.extend(0.0).normalize_or_zero() * SPEED;
    }
}

/// Advance the physics simulation by one fixed timestep. This may run zero or multiple times per frame.
///
/// Note that since this runs in `FixedUpdate`, `Res<Time>` would be `Res<Time<Fixed>>` automatically.
/// We are being explicit here for clarity.
fn advance_physics(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut PhysicalTranslation,
        &mut PreviousPhysicalTranslation,
        &mut AccumulatedInput,
        &Velocity,
    )>,
) {
    for (
        mut current_physical_translation,
        mut previous_physical_translation,
        mut input,
        velocity,
    ) in query.iter_mut()
    {
        previous_physical_translation.0 = current_physical_translation.0;
        current_physical_translation.0 += velocity.0 * fixed_time.delta_secs();

        // Reset the input accumulator, as we are currently consuming all input that happened since the last fixed timestep.
        input.0 = Vec2::ZERO;
    }
}

fn interpolate_rendered_transform(
    fixed_time: Res<Time<Fixed>>,
    mut query: Query<(
        &mut Transform,
        &PhysicalTranslation,
        &PreviousPhysicalTranslation,
    )>,
) {
    for (mut transform, current_physical_translation, previous_physical_translation) in
        query.iter_mut()
    {
        let previous = previous_physical_translation.0;
        let current = current_physical_translation.0;
        // The overstep fraction is a value between 0 and 1 that tells us how far we are between two fixed timesteps.
        let alpha = fixed_time.overstep_fraction();

        let rendered_translation = previous.lerp(current, alpha);
        transform.translation = rendered_translation;
    }
}
