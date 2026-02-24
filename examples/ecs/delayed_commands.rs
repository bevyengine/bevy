//! This example demonstrates how to send commands which will take effect after a period of time.
//!
//! We've chosen to demonstrate this effect through the creation of a grid of clickable,
//! with "ripples" created when you click.

use bevy::prelude::*;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, spawn)
        .add_observer(click)
        .run();
}

#[derive(Component)]
struct BlinkySquare;

const SQUARE_SIZE: Vec2 = Vec2::splat(45.0);

fn spawn(mut commands: Commands) {
    commands.spawn(Camera2d);
    for x in -5..=5 {
        for y in -5..=5 {
            commands.spawn((
                BlinkySquare,
                Transform::from_xyz(x as f32 * 50.0, y as f32 * 50.0, 0.0),
                Sprite::from_color(Color::BLACK, SQUARE_SIZE),
            ));
        }
    }
}

fn click(
    click: On<Pointer<Click>>,
    mut commands: Commands,
    squares: Query<(Entity, &Transform), With<BlinkySquare>>,
    cameras: Query<(&Camera, &GlobalTransform)>,
) {
    let (camera, camera_transform) = cameras.single().unwrap();
    let mut delayed = commands.delayed();
    for (entity, transform) in squares.iter() {
        // convert the pointer position to world position
        let mouse_world_pos = camera
            .viewport_to_world_2d(camera_transform, click.pointer_location.position)
            .unwrap();

        // delay the blinkiness by distance to cursor
        let dist = mouse_world_pos.distance(transform.translation.truncate());
        let delay = dist / 1000.0;
        delayed
            .secs(delay)
            .entity(entity)
            .insert(Sprite::from_color(Color::WHITE, SQUARE_SIZE));
        delayed
            .secs(delay + 0.1)
            .entity(entity)
            .insert(Sprite::from_color(Color::BLACK, SQUARE_SIZE));
    }
}
