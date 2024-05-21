//! Demonstrates how to animate colors in different color spaces using mixing and splines.

use bevy::{math::VectorSpace, prelude::*};

// We define this trait so we can reuse the same code for multiple color types that may be implemented using curves.
trait CurveColor: VectorSpace + Into<Color> + Send + Sync + 'static {}
impl<T: VectorSpace + Into<Color> + Send + Sync + 'static> CurveColor for T {}

// We define this trait so we can reuse the same code for multiple color types that may be implemented using mixing.
trait MixedColor: Mix + Into<Color> + Send + Sync + 'static {}
impl<T: Mix + Into<Color> + Send + Sync + 'static> MixedColor for T {}

#[derive(Debug, Component)]
struct Curve<T: CurveColor>(CubicCurve<T>);

#[derive(Debug, Component)]
struct Mixed<T: MixedColor>([T; 4]);

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
    commands.spawn(Camera2dBundle::default());

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
    // However, we can still mix these colours and animate that way. In fact, mixing colors works in any color space.

    // Spawn a spritre using the provided colors for mixing.
    spawn_mixed_sprite(&mut commands, -75., colors.map(Hsla::from));

    spawn_mixed_sprite(&mut commands, -175., colors.map(Srgba::from));

    spawn_mixed_sprite(&mut commands, -275., colors.map(Oklcha::from));
}

fn spawn_curve_sprite<T: CurveColor>(commands: &mut Commands, y: f32, points: [T; 4]) {
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(0., y, 0.),
            sprite: Sprite {
                custom_size: Some(Vec2::new(75., 75.)),
                ..Default::default()
            },
            ..Default::default()
        },
        Curve(CubicBezier::new([points]).to_curve()),
    ));
}

fn spawn_mixed_sprite<T: MixedColor>(commands: &mut Commands, y: f32, colors: [T; 4]) {
    commands.spawn((
        SpriteBundle {
            transform: Transform::from_xyz(0., y, 0.),
            sprite: Sprite {
                custom_size: Some(Vec2::new(75., 75.)),
                ..Default::default()
            },
            ..Default::default()
        },
        Mixed(colors),
    ));
}

fn animate_curve<T: CurveColor>(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Sprite, &Curve<T>)>,
) {
    let t = (time.elapsed_seconds().sin() + 1.) / 2.;

    for (mut transform, mut sprite, cubic_curve) in &mut query {
        // position takes a point from the curve where 0 is the initial point
        // and 1 is the last point
        sprite.color = cubic_curve.0.position(t).into();
        transform.translation.x = 600. * (t - 0.5);
    }
}

fn animate_mixed<T: MixedColor>(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &mut Sprite, &Mixed<T>)>,
) {
    let t = (time.elapsed_seconds().sin() + 1.) / 2.;

    for (mut transform, mut sprite, mixed) in &mut query {
        sprite.color = {
            // First, we determine the amount of intervals between colors.
            // For four colors, there are three intervals between those colors;
            let intervals = (mixed.0.len() - 1) as f32;

            // Next we determine the index of the first of the two colorts to mix.
            let start_i = (t * intervals).floor().min(intervals - 1.);

            // Lastly we determine the 'local' value of t in this interval.
            let local_t = (t * intervals) - start_i;

            let color = mixed.0[start_i as usize].mix(&mixed.0[start_i as usize + 1], local_t);
            color.into()
        };
        transform.translation.x = 600. * (t - 0.5);
    }
}
