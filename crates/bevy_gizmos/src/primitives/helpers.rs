use std::f32::consts::TAU;

use bevy_math::{Mat2, Quat, Vec2, Vec3};
use bevy_render::color::Color;

use crate::prelude::{GizmoConfigGroup, Gizmos};

// helpers - affine transform

pub fn rotate_then_translate_2d(rotation: Mat2, position: Vec2) -> impl Fn(Vec2) -> Vec2 {
    move |v| rotation * v + position
}

pub fn rotate_then_translate_3d(rotation: Quat, translation: Vec3) -> impl Fn(Vec3) -> Vec3 {
    move |v| rotation * v + translation
}

// helpers - circle related things

pub fn single_circle_coordinate(
    radius: f32,
    segments: usize,
    nth_point: usize,
    fraction: f32,
) -> Vec2 {
    let angle = nth_point as f32 * TAU * fraction / segments as f32;
    let (x, y) = angle.sin_cos();
    Vec2::new(x, y) * radius
}

pub fn circle_coordinates(radius: f32, segments: usize) -> impl Iterator<Item = Vec2> {
    (0..)
        .map(move |p| single_circle_coordinate(radius, segments, p, 1.0))
        .take(segments)
}

// helper - drawing

pub fn draw_cap<T: GizmoConfigGroup>(
    gizmos: &mut Gizmos<'_, '_, T>,
    radius: f32,
    segments: usize,
    rotation: Quat,
    center: Vec3,
    top: Vec3,
    color: Color,
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

pub fn draw_circle<T: GizmoConfigGroup>(
    gizmos: &mut Gizmos<'_, '_, T>,
    radius: f32,
    segments: usize,
    rotation: Quat,
    translation: Vec3,
    color: Color,
) {
    let positions = (0..=segments)
        .map(|frac| frac as f32 / segments as f32)
        .map(|percentage| percentage * TAU)
        .map(|angle| Vec2::from(angle.sin_cos()) * radius)
        .map(|p| Vec3::new(p.x, 0.0, p.y))
        .map(rotate_then_translate_3d(rotation, translation));
    gizmos.linestrip(positions, color);
}

pub fn draw_cylinder_vertical_lines<T: GizmoConfigGroup>(
    gizmos: &mut Gizmos<'_, '_, T>,
    radius: f32,
    segments: usize,
    half_height: f32,
    rotation: Quat,
    center: Vec3,
    color: Color,
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
