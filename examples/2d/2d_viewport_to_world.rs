//! This example demonstrates how to use the `Camera::viewport_to_world_2d` method.

use bevy::{color::palettes::basic::WHITE, prelude::*};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, draw_cursor)
        .run();
}

fn draw_cursor(
    camera_query: Single<(&Camera, &GlobalTransform)>,
    window: Query<&Window>,
    mut gizmos: Gizmos,
) {
    let Ok(window) = window.get_single() else {
        return;
    };

    let (camera, camera_transform) = *camera_query;

    let Some(cursor_position) = window.cursor_position() else {
        return;
    };

    // Calculate a world position based on the cursor's position.
    let Ok(point) = camera.viewport_to_world_2d(camera_transform, cursor_position) else {
        return;
    };

    gizmos.circle_2d(point, 10., WHITE);
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // Create a minimal UI explaining how to interact with the example
    commands.spawn((
        Text::new("Move the mouse to see the circle follow your cursor."),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
    ));
}
