use std::f32::consts::TAU;

use bevy_color::Color;
use bevy_math::{Mat2, Quat, Vec2, Vec3};

use crate::prelude::{GizmoConfigGroup, Gizmos};

/// Performs an isometric transformation on 2D vectors.
///
/// This function takes angle and a position vector, and returns a closure that applies
/// the isometric transformation to any given 2D vector. The transformation involves rotating
/// the vector by the specified angle and then translating it by the given position.
pub(crate) fn rotate_then_translate_2d(angle: f32, position: Vec2) -> impl Fn(Vec2) -> Vec2 {
    move |v| Mat2::from_angle(angle) * v + position
}

/// Performs an isometric transformation on 3D vectors.
///
/// This function takes a quaternion representing rotation and a 3D vector representing
/// translation, and returns a closure that applies the isometric transformation to any
/// given 3D vector. The transformation involves rotating the vector by the specified
/// quaternion and then translating it by the given translation vector.
pub(crate) fn rotate_then_translate_3d(rotation: Quat, translation: Vec3) -> impl Fn(Vec3) -> Vec3 {
    move |v| rotation * v + translation
}

/// Calculates the `nth` coordinate of a circle.
///
/// Given a circle's radiu and its resolution, this function computes the position
/// of the `nth` point along the circumference of the circle. The rotation starts at `(0.0, radius)`
/// and proceeds counter-clockwise.
pub(crate) fn single_circle_coordinate(radius: f32, resolution: u32, nth_point: u32) -> Vec2 {
    let angle = nth_point as f32 * TAU / resolution as f32;
    let (x, y) = angle.sin_cos();
    Vec2::new(x, y) * radius
}

/// Generates an iterator over the coordinates of a circle.
///
/// This function creates an iterator that yields the positions of points approximating a
/// circle with the given radius, divided into linear segments. The iterator produces `resolution`
/// number of points.
pub(crate) fn circle_coordinates(radius: f32, resolution: u32) -> impl Iterator<Item = Vec2> {
    (0..)
        .map(move |p| single_circle_coordinate(radius, resolution, p))
        .take(resolution as usize)
}

/// Draws a circle in 3D space.
///
/// # Note
///
/// This function is necessary to use instead of `gizmos.circle` for certain primitives to ensure that points align correctly. For example, the major circles of a torus are drawn with this method, and using `gizmos.circle` would result in the minor circles not being positioned precisely on the major circles' segment points.
pub(crate) fn draw_circle_3d<Config, Clear>(
    gizmos: &mut Gizmos<'_, '_, Config, Clear>,
    radius: f32,
    resolution: u32,
    rotation: Quat,
    translation: Vec3,
    color: Color,
) where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    let positions = (0..=resolution)
        .map(|frac| frac as f32 / resolution as f32)
        .map(|percentage| percentage * TAU)
        .map(|angle| Vec2::from(angle.sin_cos()) * radius)
        .map(|p| Vec3::new(p.x, 0.0, p.y))
        .map(rotate_then_translate_3d(rotation, translation));
    gizmos.linestrip(positions, color);
}
