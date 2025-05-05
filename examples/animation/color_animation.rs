//! Demonstrates how to animate colors in different color spaces using mixing and splines.

use bevy::{
    color::ColorCurve,
    math::{Curve, Interpolate},
    prelude::*,
};

// We define this trait so we can reuse the same code for multiple color types that may be implemented using curves.
trait ColorSpace: Interpolate + Into<Color> + Clone + Send + Sync + 'static {}

impl<T: Interpolate + Into<Color> + Clone + Send + Sync + 'static> ColorSpace for T {}

#[derive(Component)]
struct ExampleCurve<T: ColorSpace>(ColorCurve<T>);

#[derive(Debug, Component)]
struct ExampleInterpolate<T: ColorSpace>([T; 4]);

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                animate_curve::<LinearRgba>,
                animate_curve::<Oklaba>,
                animate_curve::<Xyza>,
                animate_mixed::<Hsla>,
                animate_mixed::<Srgba>,
                animate_mixed::<Oklcha>,
            ),
        )
        .run();
}

fn setup(mut commands: Commands) {
    commands.spawn(Camera2d);

    // The color spaces `Oklaba`, `Laba`, `LinearRgba`, `Srgba` and `Xyza` all are either perceptually or physically linear.
    // This property allows us to define curves, e.g. bezier curves through these spaces.

    // Define the control points for the curve.
    // For more information, please see the cubic curve example.
    let colors = [
        LinearRgba::WHITE,
        LinearRgba::rgb(1., 1., 0.), // Yellow
        LinearRgba::RED,
        LinearRgba::BLACK,
    ];
    // Spawn a sprite using the provided colors as control points.
    spawn_curve_sprite(&mut commands, 275., colors);

    // Spawn another sprite using the provided colors as control points after converting them to the `Xyza` color space.
    spawn_curve_sprite(&mut commands, 175., colors.map(Xyza::from));

    spawn_curve_sprite(&mut commands, 75., colors.map(Oklaba::from));

    // Other color spaces like `Srgba` or `Hsva` are neither perceptually nor physically linear.
    // As such, we cannot use curves in these spaces.
    // However, we can still mix these colors and animate that way. In fact, mixing colors works in any color space.

    // Spawn a sprite using the provided colors for mixing.
    spawn_mixed_sprite(&mut commands, -75., colors.map(Hsla::from));

    spawn_mixed_sprite(&mut commands, -175., colors.map(Srgba::from));

    spawn_mixed_sprite(&mut commands, -275., colors.map(Oklcha::from));
}

fn spawn_curve_sprite<T: ColorSpace>(commands: &mut Commands, y: f32, points: [T; 4]) {
    commands.spawn((
        Sprite::sized(Vec2::new(75., 75.)),
        Transform::from_xyz(0., y, 0.),
        ExampleCurve(ColorCurve::new(points).unwrap()),
    ));
}

fn spawn_mixed_sprite<T: ColorSpace>(commands: &mut Commands, y: f32, colors: [T; 4]) {
    commands.spawn((
        Transform::from_xyz(0., y, 0.),
        Sprite::sized(Vec2::new(75., 75.)),
        ExampleInterpolate(colors),
    ));
}

fn animate_curve<T: ColorSpace>(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Sprite, &ExampleCurve<T>)>,
) {
    let t = (ops::sin(time.elapsed_secs()) + 1.) / 2.;

    for (mut transform, mut sprite, cubic_curve) in &mut query {
        // position takes a point from the curve where 0 is the initial point
        // and 1 is the last point
        sprite.color = cubic_curve.0.sample(t * 3.).unwrap().into();
        transform.translation.x = 600. * (t - 0.5);
    }
}

fn animate_mixed<T: ColorSpace>(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Sprite, &ExampleInterpolate<T>)>,
) {
    let t = (ops::sin(time.elapsed_secs()) + 1.) / 2.;

    for (mut transform, mut sprite, mixed) in &mut query {
        sprite.color = {
            // First, we determine the amount of intervals between colors.
            // For four colors, there are three intervals between those colors;
            let intervals = (mixed.0.len() - 1) as f32;

            // Next we determine the index of the first of the two colors to mix.
            let start_i = (t * intervals).floor().min(intervals - 1.);

            // Lastly we determine the 'local' value of t in this interval.
            let local_t = (t * intervals) - start_i;

            let color = mixed.0[start_i as usize].interp(&mixed.0[start_i as usize + 1], local_t);
            color.into()
        };
        transform.translation.x = 600. * (t - 0.5);
    }
}
