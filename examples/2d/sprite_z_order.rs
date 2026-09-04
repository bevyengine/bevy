//! Shows that a blended sprite that only moves keeps the right draw order.
//!
//! Each row has a static blue square and a red square. Pressing space moves
//! every red square from behind its blue square to in front of it, or back.
//! The bottom red square is also invalidated every frame so
//! it is always drawn in the right order.

use bevy::{color::palettes::css, prelude::*, sprite::SpriteAlphaMode};

const STATIC_Z: f32 = 5.0;
const BEHIND: f32 = STATIC_Z - 1.0;
const IN_FRONT: f32 = STATIC_Z + 1.0;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(Update, (swap_on_space_sys, invalidate_reference_sys))
        .run();
}

#[derive(Component)]
struct Moving;

#[derive(Component)]
struct ToInvalidate;

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let size = Vec2::splat(140.0);
    let square = |color: Srgba| Sprite {
        alpha_mode: SpriteAlphaMode::Blend,
        ..Sprite::from_color(color.with_alpha(0.9), size)
    };

    for (row_y, invalidate_target) in [(150.0, false), (-150.0, true)] {
        commands.spawn((
            square(css::DODGER_BLUE),
            Transform::from_xyz(0.0, row_y, STATIC_Z),
        ));

        let mut red = commands.spawn((
            square(css::CRIMSON),
            Moving,
            Transform::from_xyz(60.0, row_y - 40.0, BEHIND),
        ));
        if invalidate_target {
            red.insert(ToInvalidate);
        }
    }

    commands.spawn((
        Text::new("Space: move the red squares in front of or behind the blue ones"),
        Node {
            position_type: PositionType::Absolute,
            top: px(12),
            left: px(12),
            ..default()
        },
    ));
}

fn swap_on_space_sys(
    keys: Res<ButtonInput<KeyCode>>,
    mut squares: Query<&mut Transform, With<Moving>>,
) {
    if !keys.just_pressed(KeyCode::Space) {
        return;
    }
    for mut transform in &mut squares {
        let z = &mut transform.translation.z;
        *z = if *z < STATIC_Z { IN_FRONT } else { BEHIND };
    }
}

fn invalidate_reference_sys(mut sprites: Query<&mut Sprite, With<ToInvalidate>>) {
    for mut sprite in &mut sprites {
        sprite.set_changed();
    }
}
