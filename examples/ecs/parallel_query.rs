use bevy::{prelude::*, tasks::prelude::*};
use rand::random;

struct Velocity(Vec2);

fn spawn_system(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut materials: ResMut<Assets<ColorMaterial>>,
) {
    commands.spawn(Camera2dComponents::default());
    let texture_handle = asset_server.load("assets/branding/icon.png").unwrap();
    let material = materials.add(texture_handle.into());
    for _ in 0..128 {
        commands
            .spawn(SpriteComponents {
                material,
                transform: Transform::from_scale(0.1),
                ..Default::default()
            })
            .with(Velocity(
                20.0 * Vec2::new(random::<f32>() - 0.5, random::<f32>() - 0.5),
            ));
    }
}

// Move sprites according to their velocity
fn move_system(pool: Res<ComputeTaskPool>, mut sprites: Query<(&mut Transform, &Velocity)>) {
    // Compute the new location of each sprite in parallel on the
    // ComputeTaskPool using batches of 32 sprties
    //
    // This example is only for demonstrative purposes.  Using a
    // ParallelIterator for an inexpensive operation like addition on only 128
    // elements will not typically be faster than just using a normal Iterator.
    // See the ParallelIterator documentation for more information on when
    // to use or not use ParallelIterator over a normal Iterator.
    sprites
        .iter()
        .par_iter(32)
        .for_each(&pool, |(mut transform, velocity)| {
            transform.translate(velocity.0.extend(0.0));
        });
}

// Bounce sprties outside the window
fn bounce_system(
    pool: Res<ComputeTaskPool>,
    windows: Res<Windows>,
    mut sprites: Query<(&Transform, &mut Velocity)>,
) {
    let Window { width, height, .. } = windows.get_primary().expect("No primary window");
    let left = *width as f32 / -2.0;
    let right = *width as f32 / 2.0;
    let bottom = *height as f32 / -2.0;
    let top = *height as f32 / 2.0;
    sprites
        .iter()
        // Batch size of 32 is chosen to limit the overhead of
        // ParallelIterator, since negating a vector is very inexpensive.
        .par_iter(32)
        // Filter out sprites that don't need to be bounced
        .filter(|(transform, _)| {
            !(left < transform.translation().x()
                && transform.translation().x() < right
                && bottom < transform.translation().y()
                && transform.translation().y() < top)
        })
        // For simplicity, just reverse the velocity; don't use realistic bounces
        .for_each(&pool, |(_, mut v)| {
            v.0 = -v.0;
        });
}

fn main() {
    App::build()
        .add_default_plugins()
        .add_startup_system(spawn_system.system())
        .add_system(move_system.system())
        .add_system(bounce_system.system())
        .run();
}
