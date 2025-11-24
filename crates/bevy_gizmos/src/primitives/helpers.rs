use core::f32::consts::TAU;

use bevy_math::{ops, Vec2};

/// Calculates the `nth` coordinate of a circle.
///
/// Given a circle's radius and its resolution, this function computes the position
/// of the `nth` point along the circumference of the circle. The rotation starts at `(0.0, radius)`
/// and proceeds counter-clockwise.
pub(crate) fn single_circle_coordinate(radius: f32, resolution: u32, nth_point: u32) -> Vec2 {
    let angle = nth_point as f32 * TAU / resolution as f32;
    let (x, y) = ops::sin_cos(angle);
    Vec2::new(x, y) * radius
}

/// Generates an iterator over the coordinates of a circle.
///
/// The coordinates form an open circle, meaning the first and last points aren't the same.
///
/// This function creates an iterator that yields the positions of points approximating a
/// circle with the given radius, divided into linear segments. The iterator produces `resolution`
/// number of points.
pub(crate) fn circle_coordinates(radius: f32, resolution: u32) -> impl Iterator<Item = Vec2> {
    (0..)
        .map(move |p| single_circle_coordinate(radius, resolution, p))
        .take(resolution as usize)
}

/// Generates an iterator over the coordinates of a circle.
///
/// The coordinates form a closed circle, meaning the first and last points are the same.
///
/// This function creates an iterator that yields the positions of points approximating a
/// circle with the given radius, divided into linear segments. The iterator produces `resolution`
/// number of points.
pub(crate) fn circle_coordinates_closed(
    radius: f32,
    resolution: u32,
) -> impl Iterator<Item = Vec2> {
    circle_coordinates(radius, resolution).chain(core::iter::once(single_circle_coordinate(
        radius, resolution, resolution,
    )))
}
