//! Demonstrates picking for text2d. The picking backend only tests against the
//! text2d bounds, so the 2d text can be picked by clicking on its transparent areas.

use bevy::{prelude::*, sprite::Anchor};
use std::fmt::Debug;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(ImagePlugin::default_nearest()))
        .add_systems(Startup, setup)
        .add_systems(Update, move_text2d)
        .run();
}

fn move_text2d(
    time: Res<Time>,
    mut sprite: Query<&mut Transform, (Without<Sprite>, Without<Text2d>, With<Children>)>,
) {
    let t = time.elapsed_secs() * 0.1;
    for mut transform in &mut sprite {
        let new = Vec2 {
            x: 50.0 * ops::sin(t),
            y: 50.0 * ops::sin(t * 2.0),
        };
        transform.translation.x = new.x;
        transform.translation.y = new.y;
    }
}

/// Set up a scene that tests all anchor types.
fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    let w = 256.0;
    let h = 128.0;

    commands
        .spawn((Transform::default(), Visibility::default()))
        .with_children(|commands| {
            for (anchor_index, anchor) in [
                Anchor::TopLeft,
                Anchor::TopCenter,
                Anchor::TopRight,
                Anchor::CenterLeft,
                Anchor::Center,
                Anchor::CenterRight,
                Anchor::BottomLeft,
                Anchor::BottomCenter,
                Anchor::BottomRight,
                Anchor::Custom(Vec2::new(0.5, 0.5)),
            ]
            .iter()
            .enumerate()
            {
                let i = (anchor_index % 3) as f32;
                let j = (anchor_index / 3) as f32;

                // spawn black square behind text to show anchor point
                commands.spawn((
                    Sprite::from_color(Color::BLACK, Vec2::splat(15.0)),
                    Transform::from_xyz(i * w - w, -1.0 * (j * h - h), -1.0),
                ));

                commands
                    .spawn((
                        Text2d(format!("{:?}", anchor)),
                        TextColor(Color::WHITE),
                        *anchor,
                        // 3x3 grid of anchor examples by changing transform
                        Transform::from_xyz(i * w - w, -1.0 * (j * h - h), 0.0)
                            .with_scale(Vec3::splat(1.0 + (i - 1.0) * 0.2))
                            .with_rotation(Quat::from_rotation_z((j - 1.0) * 0.2)),
                    ))
                    .observe(recolor_on::<Pointer<Over>>(Color::srgb(0.0, 1.0, 0.0)))
                    .observe(recolor_on::<Pointer<Out>>(Color::srgb(1.0, 0.0, 0.0)))
                    .observe(recolor_on::<Pointer<Pressed>>(Color::srgb(0.0, 0.0, 1.0)))
                    .observe(recolor_on::<Pointer<Released>>(Color::srgb(0.0, 1.0, 0.0)));
            }
        });
}

// An observer listener that changes the target entity's color.
fn recolor_on<E: Debug + Clone + Reflect>(
    color: Color,
) -> impl Fn(Trigger<E>, Query<&mut TextColor>) {
    move |ev, mut text_colors| {
        let Ok(mut text_color) = text_colors.get_mut(ev.target()) else {
            return;
        };
        text_color.0 = color;
    }
}
