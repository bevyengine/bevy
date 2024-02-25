use std::f32::consts::TAU;

use bevy_math::{Mat2, Quat, Vec2, Vec3};
use bevy_render::color::LegacyColor;

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

/// Calculates the `nth` coordinate of a circle segment.
///
/// Given a circle's radiu and the number of segments, this function computes the position
/// of the `nth` point along the circumference of the circle. The rotation starts at `(0.0, radius)`
/// and proceeds counter-clockwise.
pub(crate) fn single_circle_coordinate(radius: f32, segments: usize, nth_point: usize) -> Vec2 {
    let angle = nth_point as f32 * TAU / segments as f32;
    let (x, y) = angle.sin_cos();
    Vec2::new(x, y) * radius
}

/// Generates an iterator over the coordinates of a circle segment.
///
/// This function creates an iterator that yields the positions of points approximating a
/// circle with the given radius, divided into linear segments. The iterator produces `segments`
/// number of points.
pub(crate) fn circle_coordinates(radius: f32, segments: usize) -> impl Iterator<Item = Vec2> {
    (0..)
        .map(move |p| single_circle_coordinate(radius, segments, p))
        .take(segments)
}

/// Draws a semi-sphere.
///
/// This function draws a semi-sphere at the specified `center` point with the given `rotation`,
/// `radius`, and `color`. The `segments` parameter determines the level of detail, and the `top`
/// argument specifies the shape of the semi-sphere's tip.
pub(crate) fn draw_semi_sphere<T: GizmoConfigGroup>(
    gizmos: &mut Gizmos<'_, '_, T>,
    radius: f32,
    segments: usize,
    rotation: Quat,
    center: Vec3,
    top: Vec3,
    color: LegacyColor,
) {
    circle_coordinates(radius, segments)
        .map(|p| Vec3::new(p.x, 0.0, p.y))
        .map(rotate_then_translate_3d(rotation, center))
        .for_each(|from| {
            gizmos
                .short_arc_3d_between(center, from, top, color)
                .segments(segments / 2);
        });
}

/// Draws a circle in 3D space.
///
/// # Note
///
/// This function is necessary to use instead of `gizmos.circle` for certain primitives to ensure that points align correctly. For example, the major circles of a torus are drawn with this method, and using `gizmos.circle` would result in the minor circles not being positioned precisely on the major circles' segment points.
pub(crate) fn draw_circle_3d<T: GizmoConfigGroup>(
    gizmos: &mut Gizmos<'_, '_, T>,
    radius: f32,
    segments: usize,
    rotation: Quat,
    translation: Vec3,
    color: LegacyColor,
) {
    let positions = (0..=segments)
        .map(|frac| frac as f32 / segments as f32)
        .map(|percentage| percentage * TAU)
        .map(|angle| Vec2::from(angle.sin_cos()) * radius)
        .map(|p| Vec3::new(p.x, 0.0, p.y))
        .map(rotate_then_translate_3d(rotation, translation));
    gizmos.linestrip(positions, color);
}

/// Draws the connecting lines of a cylinder between the top circle and the bottom circle.
pub(crate) fn draw_cylinder_vertical_lines<T: GizmoConfigGroup>(
    gizmos: &mut Gizmos<'_, '_, T>,
    radius: f32,
    segments: usize,
    half_height: f32,
    rotation: Quat,
    center: Vec3,
    color: LegacyColor,
) {
    circle_coordinates(radius, segments)
        .map(move |point_2d| {
            [1.0, -1.0]
                .map(|sign| sign * half_height)
                .map(|height| Vec3::new(point_2d.x, height, point_2d.y))
        })
        .map(|ps| ps.map(rotate_then_translate_3d(rotation, center)))
        .for_each(|[start, end]| {
            gizmos.line(start, end, color);
        });
}
