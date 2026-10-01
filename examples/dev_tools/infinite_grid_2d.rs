//! Shows how to use an infinite grid in a 2D scene.

use std::ops::RangeInclusive;

use bevy::{
    dev_tools::infinite_grid::{InfiniteGrid, InfiniteGridPlugin, InfiniteGridSettings},
    input::mouse::{AccumulatedMouseScroll, MouseScrollPixelsPerLine},
    prelude::*,
};

fn main() {
    App::new()
        .add_plugins((DefaultPlugins, InfiniteGridPlugin))
        .add_systems(Startup, setup)
        .add_systems(Update, zoom)
        .run();
}

fn setup(mut commands: Commands, asset_server: Res<AssetServer>) {
    commands.spawn((
        InfiniteGrid,
        InfiniteGridSettings {
            // The grid was originally made for the XZ plane so we need to rotate it and
            // recolor the Z axis
            // This color is the Y_AXIS in bevy_feathers
            z_axis_color: Color::oklcha(0.5866, 0.1543, 129.84, 1.0),
            // Distance between minor lines is 1/scale so this results in 1 minor line every 10
            // world unit. In 2d a world unit is 1 pixel
            scale: 0.1,
            major_line_interval: 10,
            // Disable fadeout because it doesn't make sense in 2d
            dot_fadeout_strength: 0.0,
            ..default()
        },
        // Draw the grid behind the sprites
        Transform::from_xyz(0.0, 0.0, -1.0)
            // The grid defaults to be on the XZ plane which doesn't work in 2d so we need to rotate it
            // so it's on the XY plane
            .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
    ));

    commands.spawn(Camera2d);

    commands.spawn((
        Text::new("Use scroll to zoom"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));

    commands.spawn(Sprite::from_image(
        asset_server.load("branding/bevy_bird_dark.png"),
    ));
}

fn zoom(
    mut projection: Single<&mut Projection, With<Camera2d>>,
    mouse_scroll: Res<AccumulatedMouseScroll>,
    scroll_conversion: Res<MouseScrollPixelsPerLine>,
) {
    let scroll = mouse_scroll.to_lines(&scroll_conversion).delta.y;
    if scroll == 0.0 {
        return;
    }
    let Projection::Orthographic(orthographic) = &mut **projection else {
        return;
    };
    let scroll_speed = 0.1;
    orthographic.scale = (orthographic.scale * (1.0 - scroll * scroll_speed)).clamp(0.1, 5.0);
}
