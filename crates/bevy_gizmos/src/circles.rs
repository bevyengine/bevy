//! Additional [`Gizmos`] Functions -- Circles
//!
//! Includes the implementation of [`Gizmos::circle`] and [`Gizmos::circle_2d`],
//! and assorted support items.

use crate::prelude::Gizmos;
use bevy_math::{Quat, Vec2, Vec3};
use bevy_render::color::Color;
use std::f32::consts::TAU;

pub(crate) const DEFAULT_CIRCLE_SEGMENTS: usize = 32;

fn circle_inner(radius: f32, segments: usize) -> impl Iterator<Item = Vec2> {
    (0..segments + 1).map(move |i| {
        let angle = i as f32 * TAU / segments as f32;
        Vec2::from(angle.sin_cos()) * radius
    })
}

impl<'s> Gizmos<'s> {
    /// Draw a circle in 3D at `position` with the flat side facing `normal`.
    ///
    /// This should be called for each frame the circle needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.circle(Vec3::ZERO, Vec3::Z, 1., Color::GREEN);
    ///
    ///     // Circles have 32 line-segments by default.
    ///     // You may want to increase this for larger circles.
    ///     gizmos
    ///         .circle(Vec3::ZERO, Vec3::Z, 5., Color::RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn circle(
        &mut self,
        position: Vec3,
        normal: Vec3,
        radius: f32,
        color: Color,
    ) -> CircleBuilder<'_, 's> {
        CircleBuilder {
            gizmos: self,
            position,
            normal,
            radius,
            color,
            segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw a circle in 2D.
    ///
    /// This should be called for each frame the circle needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.circle_2d(Vec2::ZERO, 1., Color::GREEN);
    ///
    ///     // Circles have 32 line-segments by default.
    ///     // You may want to increase this for larger circles.
    ///     gizmos
    ///         .circle_2d(Vec2::ZERO, 5., Color::RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn circle_2d(
        &mut self,
        position: Vec2,
        radius: f32,
        color: Color,
    ) -> Circle2dBuilder<'_, 's> {
        Circle2dBuilder {
            gizmos: self,
            position,
            radius,
            color,
            segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }
}

/// A builder returned by [`Gizmos::circle`].
pub struct CircleBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,
    position: Vec3,
    normal: Vec3,
    radius: f32,
    color: Color,
    segments: usize,
}

impl CircleBuilder<'_, '_> {
    /// Set the number of line-segments for this circle.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl Drop for CircleBuilder<'_, '_> {
    fn drop(&mut self) {
        let rotation = Quat::from_rotation_arc(Vec3::Z, self.normal);
        let positions = circle_inner(self.radius, self.segments)
            .map(|vec2| self.position + rotation * vec2.extend(0.));
        self.gizmos.linestrip(positions, self.color);
    }
}

/// A builder returned by [`Gizmos::circle_2d`].
pub struct Circle2dBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,
    position: Vec2,
    radius: f32,
    color: Color,
    segments: usize,
}

impl Circle2dBuilder<'_, '_> {
    /// Set the number of line-segments for this circle.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl Drop for Circle2dBuilder<'_, '_> {
    fn drop(&mut self) {
        let positions = circle_inner(self.radius, self.segments).map(|vec2| vec2 + self.position);
        self.gizmos.linestrip_2d(positions, self.color);
    }
}
