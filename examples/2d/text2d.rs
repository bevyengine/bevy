//! Shows text rendering with moving, rotating and scaling text.
//!
//! Note that this uses [`Text2d`] to display text alongside your other entities in a 2D scene.
//!
//! For an example on how to render text as part of a user interface, independent from the world
//! viewport, you may want to look at `games/contributors.rs` or `ui/text.rs`.

use bevy::color::palettes;
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::TextBounds;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .run();
}

fn example(commands: &mut Commands, dest: Vec3, justify: Justify) {
    commands.spawn((
        Sprite::from_color(palettes::css::YELLOW, 10. * Vec2::ONE),
        Anchor::CENTER,
        Transform::from_translation(dest),
    ));

    for (a, bg) in [
        (Anchor::TOP_LEFT, palettes::css::DARK_SLATE_GREY),
        (Anchor::TOP_RIGHT, palettes::css::DARK_OLIVEGREEN),
        (Anchor::BOTTOM_RIGHT, palettes::css::DARK_SLATE_GREY),
        (Anchor::BOTTOM_LEFT, palettes::css::DARK_OLIVEGREEN),
    ] {
        commands.spawn((
            Sprite::from_color(bg, Vec2::new(300., 75.)),
            a,
            Transform::from_translation(dest - Vec3::Z),
        ));

        commands.spawn((
            Text2d(format!("L R\n{:?}\n{:?}", a.0, justify)),
            TextFont {
                font_size: 14.0,
                ..default()
            },
            TextLayout {
                justify,
                ..Default::default()
            },
            TextBounds::new(300., 75.),
            Transform::from_translation(dest + Vec3::Z),
            a,
            ShowAabbGizmo::default(),
        ));
    }
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d::default());

    for (i, j) in [
        Justify::Left,
        Justify::Right,
        Justify::Center,
        Justify::Justified,
    ]
    .into_iter()
    .enumerate()
    {
        example(&mut commands, (240. - 160. * i as f32) * Vec3::Y, j);
    }

    commands.spawn((
        Sprite::from_color(palettes::css::GREEN, 10. * Vec2::ONE),
        Anchor::CENTER,
    ));
}
