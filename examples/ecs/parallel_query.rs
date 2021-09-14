use bevy::{prelude::*, tasks::prelude::*};
use rand::random;

struct Velocity(Vec2);

fn spawn_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn_bundle(OrthographicCameraBundle::new_2d());
    let texture_handle = asset_server.load("branding/icon.png");
    let material = materials.add(texture_handle.into());
    for _ in 0..128 {
        commands
            .spawn_bundle(SpriteBundle {
                material: material.clone(),
                transform: Transform::from_scale(Vec3::splat(0.1)),
                ..Default::default()
            })
            .insert(Velocity(
                20.0 * Vec2::new(random::<f32>() - 0.5, random::<f32>() - 0.5),
            ));
    }
}

// Move sprites according to their velocity
fn move_system(pool: Res<ComputeTaskPool>, mut sprites: Query<(&mut Transform, &Velocity)>) {
    // Compute the new location of each sprite in parallel on the
    // ComputeTaskPool using batches of 32 sprites
    //
    // This example is only for demonstrative purposes.  Using a
    // ParallelIterator for an inexpensive operation like addition on only 128
    // elements will not typically be faster than just using a normal Iterator.
    // See the ParallelIterator documentation for more information on when
    // to use or not use ParallelIterator over a normal Iterator.
    sprites.par_for_each_mut(&pool, 32, |(mut transform, velocity)| {
        transform.translation += velocity.0.extend(0.0);
    });
}

// Bounce sprites outside the window
fn bounce_system(
    pool: Res<ComputeTaskPool>,
    windows: Res<Windows>,
    mut sprites: Query<(&Transform, &mut Velocity)>,
) {
    let window = windows.get_primary().expect("No primary window.");
    let width = window.width();
    let height = window.height();
    let left = width / -2.0;
    let right = width / 2.0;
    let bottom = height / -2.0;
    let top = height / 2.0;
    sprites
        // Batch size of 32 is chosen to limit the overhead of
        // ParallelIterator, since negating a vector is very inexpensive.
        .par_for_each_mut(&pool, 32, |(transform, mut v)| {
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
        .add_startup_system(spawn_system)
        .add_system(move_system)
        .add_system(bounce_system)
        .run();
}
