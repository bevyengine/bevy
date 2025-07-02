//! Demonstrates sprite rendering order using `ZIndex`, `YSort`, and `SortBias` components.
//!
//! This example shows how different sorting methods interact:
//! - `bevy::sprite::ZIndex(i32)`: Absolute rendering order (higher = on top)
//! - `YSort`: Automatic sorting based on Y position (lower Y = behind)
//! - `SortBias`: Fine-tune sorting without changing actual position

use bevy::sprite::YSort;
use bevy::{prelude::*, sprite::SortBias};

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (move_sprites, update_info_text))
        .run();
}

#[derive(Component)]
struct Movable {
    label: String,
}

#[derive(Component)]
struct InfoText;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    commands.spawn((
        Text::new(
            "WASD: Move white sprite | Q/E: Adjust sort bias\n\
            Hover over sprites to see their sorting properties",
        ),
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(12.0),
            left: Val::Px(12.0),
            ..default()
        },
        InfoText,
    ));

    // Left

    commands.spawn((
        Sprite {
            color: Color::srgb(0.8, 0.2, 0.2),
            custom_size: Some(Vec2::splat(60.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(-300.0, 50.0, 0.0)),
        bevy::sprite::ZIndex(10),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.2, 0.8),
            custom_size: Some(Vec2::splat(60.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(-280.0, 30.0, 0.0)),
        bevy::sprite::ZIndex(5),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.8, 0.2),
            custom_size: Some(Vec2::splat(60.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(-260.0, 70.0, 0.0)),
        bevy::sprite::ZIndex(15),
    ));

    // Center

    for i in 0..3 {
        let y = -50.0 + i as f32 * 40.0;
        commands.spawn((
            Sprite {
                color: Color::srgb(0.9, 0.5, 0.1),
                custom_size: Some(Vec2::splat(60.0)),
                ..default()
            },
            Transform::from_translation(Vec3::new(-50.0 + i as f32 * 20.0, y, 0.0)),
            YSort,
        ));
    }

    // Right

    commands.spawn((
        Sprite {
            color: Color::srgb(0.6, 0.2, 0.8),
            custom_size: Some(Vec2::splat(60.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(200.0, 0.0, 0.0)),
        YSort,
        SortBias(-20.0),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgb(0.2, 0.8, 0.8),
            custom_size: Some(Vec2::splat(60.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(220.0, 10.0, 0.0)),
        YSort,
        SortBias(20.0),
    ));

    commands.spawn((
        Sprite {
            color: Color::srgb(0.8, 0.8, 0.2),
            custom_size: Some(Vec2::splat(60.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(240.0, 0.0, 0.0)),
        YSort,
    ));

    // Moveable sprite
    commands.spawn((
        Sprite {
            color: Color::WHITE,
            custom_size: Some(Vec2::splat(50.0)),
            ..default()
        },
        Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
        YSort,
        SortBias(0.0),
        Movable {
            label: "Movable (YSort + Bias: 0)".to_string(),
        },
    ));

    // Background grid
    for x in -4..=4 {
        for y in -3..=3 {
            commands.spawn((
                Sprite {
                    color: Color::srgba(0.3, 0.3, 0.3, 0.1),
                    custom_size: Some(Vec2::splat(30.0)),
                    ..default()
                },
                Transform::from_translation(Vec3::new(x as f32 * 100.0, y as f32 * 100.0, 0.0)),
                bevy::sprite::ZIndex(-100), // Always behind everything
            ));
        }
    }

    // Labels

    commands.spawn((
        Text::new("ZIndex Only"),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(300.0),
            bottom: Val::Px(50.0),
            ..default()
        },
        bevy::sprite::ZIndex(1000),
    ));

    commands.spawn((
        Text::new("YSort Only"),
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(550.0),
            bottom: Val::Px(50.0),
            ..default()
        },
        bevy::sprite::ZIndex(1000),
    ));

    commands.spawn((
        Text::new("YSort + SortBias"),
        Node {
            position_type: PositionType::Absolute,
            right: Val::Px(300.0),
            bottom: Val::Px(50.0),
            ..default()
        },
        bevy::sprite::ZIndex(1000),
    ));
}

fn move_sprites(
    mut query: Query<(&mut Transform, &mut SortBias, &mut Movable)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
) {
    let speed = 150.0;
    let bias_speed = 50.0;
    let delta = time.delta_secs();

    for (mut transform, mut sort_bias, mut movable) in &mut query {
        if keyboard.pressed(KeyCode::KeyA) {
            transform.translation.x -= speed * delta;
        }
        if keyboard.pressed(KeyCode::KeyD) {
            transform.translation.x += speed * delta;
        }
        if keyboard.pressed(KeyCode::KeyW) {
            transform.translation.y += speed * delta;
        }
        if keyboard.pressed(KeyCode::KeyS) {
            transform.translation.y -= speed * delta;
        }

        // Adjust sort bias
        if keyboard.pressed(KeyCode::KeyQ) {
            sort_bias.0 -= bias_speed * delta;
        }
        if keyboard.pressed(KeyCode::KeyE) {
            sort_bias.0 += bias_speed * delta;
        }

        // Update label
        movable.label = format!(
            "Movable (Y: {:.0}, Bias: {:.0})",
            transform.translation.y, sort_bias.0
        );
    }
}

fn update_info_text(mut text: Single<&mut Text, With<InfoText>>, movable: Single<&Movable>) {
    text.0 = format!(
        "WASD: Move white sprite | Q/E: Adjust sort bias\n{}",
        movable.label
    );
}
