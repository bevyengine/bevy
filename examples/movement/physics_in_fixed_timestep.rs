//! TODO
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
        .add_systems(FixedUpdate, advance_physics)
        .add_systems(
            Update,
            (update_displayed_transform, reset_velocity, handle_input).chain(),
        )
        .run();
}

#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct Velocity(Vec2);

#[derive(Debug, Component, Clone, Copy, PartialEq, Default, Deref, DerefMut)]
struct PhysicalTranslation(Vec2);

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

///
fn reset_velocity(mut query: Query<&mut Velocity>) {
    for mut velocity in query.iter_mut() {
        velocity.0 = Vec2::ZERO;
    }
}

/// Handle keyboard input to move the player.
/// Note that this should *not* change the player's position directly.
fn handle_input(keyboard_input: Res<ButtonInput<KeyCode>>, mut query: Query<&mut Velocity>) {
    /// Since Bevy's default 2D camera setup is scaled such that
    /// one unit is one pixel, you can think of this as
    /// "How many pixels per second should the player move?"
    const SPEED: f32 = 210.0;
    for mut velocity in query.iter_mut() {
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
        let alpha = fixed_time.overstep_fraction();

        // Lerp between the last displayed translation and the actual translation
        // This is needed because the `advance_physics` system may run zero or multiple times per frame,
        // so the visual amount of movement could be quite jumpy in the worst case. Imagine
        // the physics not advancing for a frame, then advancing 5 times the next frame.
        let next_displayed_translation = last_displayed_translation.lerp(actual_translation, alpha);

        transform.translation = next_displayed_translation;
    }
}
