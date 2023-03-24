//! Illustrates parallel queries with `ParallelIterator`.

use bevy::ecs::query::BatchingStrategy;
use bevy::prelude::*;
use rand::random;

#[derive(Component, Deref)]
struct Velocity(Vec2);

fn spawn_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn(Camera2dBundle::default());
    let texture = asset_server.load("branding/icon.png");
    for _ in 0..128 {
        commands.spawn((
            SpriteBundle {
                texture: texture.clone(),
                transform: Transform::from_scale(Vec3::splat(0.1)),
                ..default()
            },
            Velocity(20.0 * Vec2::new(random::<f32>() - 0.5, random::<f32>() - 0.5)),
        ));
    }
}

// Move sprites according to their velocity
fn move_system(mut sprites: Query<(&mut Transform, &Velocity)>) {
    // Compute the new location of each sprite in parallel on the
    // ComputeTaskPool
    //
    // This example is only for demonstrative purposes. Using a
    // ParallelIterator for an inexpensive operation like addition on only 128
    // elements will not typically be faster than just using a normal Iterator.
    // See the ParallelIterator documentation for more information on when
    // to use or not use ParallelIterator over a normal Iterator.
    sprites
        .par_iter_mut()
        .for_each_mut(|(mut transform, velocity)| {
            transform.translation += velocity.extend(0.0);
        });
}

// Bounce sprites outside the window
fn bounce_system(windows: Query<&Window>, mut sprites: Query<(&Transform, &mut Velocity)>) {
    let window = windows.single();
    let width = window.width();
    let height = window.height();
    let left = width / -2.0;
    let right = width / 2.0;
    let bottom = height / -2.0;
    let top = height / 2.0;
    // The default batch size can also be overridden.
    // In this case a batch size of 32 is chosen to limit the overhead of
    // ParallelIterator, since negating a vector is very inexpensive.
    sprites
        .par_iter_mut()
        .batching_strategy(BatchingStrategy::fixed(32))
        .for_each_mut(|(transform, mut v)| {
            if !(left < transform.translation.x
                && transform.translation.x < right
                && bottom < transform.translation.y
                && transform.translation.y < top)
            {
                // For simplicity, just reverse the velocity; don't use realistic bounces
                v.0 = -v.0;
            }
        });
}

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, spawn_system)
        .add_systems(Update, (move_system, bounce_system))
        .run();
}
