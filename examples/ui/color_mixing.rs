//! Demonstrates the differences in blending colors depending on which space the operation
//! is performed in.
//!
//! This demo cycles through a quadratic spline over `Color::RED`, `Color::GREEN`, and
//! `Color::BLUE` using a `sin()` driver.

use bevy::{prelude::*, render::color::palette};

#[derive(Component, Default)]
struct Slider<T>(std::marker::PhantomData<T>);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, |mut commands: Commands| {
            commands.spawn(Camera2dBundle::default());

            commands
                .spawn(NodeBundle {
                    style: Style {
                        display: Display::Grid,
                        width: Val::Percent(100.0),
                        height: Val::Percent(100.0),
                        grid_template_columns: std::iter::repeat(GridTrack::auto())
                            .take(5)
                            .collect(),
                        grid_template_rows: vec![GridTrack::auto()],
                        ..default()
                    },
                    background_color: BackgroundColor(Color::WHITE),
                    ..default()
                })
                .with_children(|builder| {
                    builder
                        .spawn((NodeBundle::default(), Slider::<palette::Srgb>::default()))
                        .with_children(|builder| {
                            builder.spawn(TextBundle::from_section("sRGB", default()));
                        });
                    builder
                        .spawn((NodeBundle::default(), Slider::<palette::LinSrgb>::default()))
                        .with_children(|builder| {
                            builder.spawn(TextBundle::from_section("Linear sRGB", default()));
                        });
                    builder
                        .spawn((NodeBundle::default(), Slider::<palette::Hsl>::default()))
                        .with_children(|builder| {
                            builder.spawn(TextBundle::from_section("HSL", default()));
                        });
                    builder
                        .spawn((NodeBundle::default(), Slider::<palette::Lch>::default()))
                        .with_children(|builder| {
                            builder.spawn(TextBundle::from_section("Lch", default()));
                        });
                    builder
                        .spawn((NodeBundle::default(), Slider::<palette::Oklch>::default()))
                        .with_children(|builder| {
                            builder.spawn(TextBundle::from_section("Oklch", default()));
                        });
                });
        })
        .add_systems(
            Update,
            (
                slider_updater::<palette::Srgb>,
                slider_updater::<palette::LinSrgb>,
                slider_updater::<palette::Hsl>,
                slider_updater::<palette::Lch>,
                slider_updater::<palette::Oklch>,
            ),
        )
        .run();
}

fn slider_updater<T>(mut query: Query<&mut BackgroundColor, With<Slider<T>>>, time: Res<Time>)
where
    T: Send
        + Sync
        + 'static
        + palette::Mix<Scalar = f32>
        + palette::convert::FromColorUnclamped<Color>
        + palette::Clamp,
    Color: palette::convert::FromColorUnclamped<T>,
{
    let t = 0.5 + 0.5 * time.elapsed_seconds_wrapped().sin();

    const P1: Color = Color::RED;
    const P2: Color = Color::GREEN;
    const P3: Color = Color::BLUE;

    let p12 = P1.mix_in::<T>(P2, t);
    let p23 = P2.mix_in::<T>(P3, t);

    let new_color = p12.mix_in::<T>(p23, t);

    for mut old_color in query.iter_mut() {
        // Example of using perceptual equality to avoid color mutation
        if !new_color.perceptually_eq(old_color.0) {
            old_color.0 = new_color;
        }
    }
}
