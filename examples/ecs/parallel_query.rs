use bevy::{math::Vec3Swizzles, prelude::*, tasks::prelude::*};
use rand::random;

#[derive(Component, Deref)]
struct Velocity(Vec2);

fn spawn_system(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    let texture = asset_server.load("branding/icon.png");
    for _ in 0..8192 {
        commands
            .spawn_bundle(SpriteBundle {
                texture: texture.clone(),
                transform: Transform::from_scale(Vec3::splat(0.1)),
                ..default()
            })
            .insert(Velocity(
                20.0 * Vec2::new(random::<f32>() - 0.5, random::<f32>() - 0.5),
            ));
    }
}

// Move sprites according to their velocity
fn move_system(pool: Res<ComputeTaskPool>, mut sprites: Query<(&mut Transform, &Velocity)>) {
    // Compute the new location of each sprite in parallel on the
    // ComputeTaskPool using batches of 128 sprites
    //
    // This example is only for demonstrative purposes. Using par_for_each
    // for an inexpensive operation like addition on only a small number of
    // elements will not typically be faster than just using for_each.
    // See the par_for_each documentation for more information on when
    // to use or not use par_for_each over a normal for_each.
    sprites.par_for_each_mut(&pool, 128, |(mut transform, velocity)| {
        transform.translation += velocity.extend(0.0);
    });
}

// Bounce sprites outside the window
fn bounce_system(
    pool: Res<ComputeTaskPool>,
    windows: Res<Windows>,
    mut sprites: Query<(&Transform, &mut Velocity)>,
) {
    let window = windows.primary();
    let width = window.width();
    let height = window.height();
    let left = width / -2.0;
    let right = width / 2.0;
    let bottom = height / -2.0;
    let top = height / 2.0;
    sprites
        // Batch size of 128 is chosen to limit the overhead of
        // par_for_each, since negating a Vec2 is very inexpensive.
        .par_for_each_mut(&pool, 128, |(transform, mut v)| {
            if v.0.dot(transform.translation.xy()) > 0.0
                && !(left < transform.translation.x
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
        .add_startup_system(spawn_system)
        .add_system(move_system)
        .add_system(bounce_system)
        .run();
}
