//! Additional [`Gizmos`] Functions -- Circles
//!
//! Includes the implementation of [`Gizmos::circle`] and [`Gizmos::circle_2d`],
//! and assorted support items.

use crate::prelude::{GizmoConfigGroup, Gizmos};
use bevy_color::Color;
use bevy_math::Mat2;
use bevy_math::{Dir3, Quat, Vec2, Vec3};
use std::f32::consts::TAU;

pub(crate) const DEFAULT_CIRCLE_SEGMENTS: usize = 32;

fn ellipse_inner(half_size: Vec2, segments: usize) -> impl Iterator<Item = Vec2> {
    (0..segments + 1).map(move |i| {
        let angle = i as f32 * TAU / segments as f32;
        let (x, y) = angle.sin_cos();
        Vec2::new(x, y) * half_size
    })
}

impl<'w, 's, Config, Clear> Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draw an ellipse in 3D at `position` with the flat side facing `normal`.
    ///
    /// This should be called for each frame the ellipse needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ellipse(Vec3::ZERO, Quat::IDENTITY, Vec2::new(1., 2.), GREEN);
    ///
    ///     // Ellipses have 32 line-segments by default.
    ///     // You may want to increase this for larger ellipses.
    ///     gizmos
    ///         .ellipse(Vec3::ZERO, Quat::IDENTITY, Vec2::new(5., 1.), RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ellipse(
        &mut self,
        position: Vec3,
        rotation: Quat,
        half_size: Vec2,
        color: impl Into<Color>,
    ) -> EllipseBuilder<'_, 'w, 's, Config, Clear> {
        EllipseBuilder {
            gizmos: self,
            position,
            rotation,
            half_size,
            color: color.into(),
            segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw an ellipse in 2D.
    ///
    /// This should be called for each frame the ellipse needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ellipse_2d(Vec2::ZERO, 180.0_f32.to_radians(), Vec2::new(2., 1.), GREEN);
    ///
    ///     // Ellipses have 32 line-segments by default.
    ///     // You may want to increase this for larger ellipses.
    ///     gizmos
    ///         .ellipse_2d(Vec2::ZERO, 180.0_f32.to_radians(), Vec2::new(5., 1.), RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ellipse_2d(
        &mut self,
        position: Vec2,
        angle: f32,
        half_size: Vec2,
        color: impl Into<Color>,
    ) -> Ellipse2dBuilder<'_, 'w, 's, Config, Clear> {
        Ellipse2dBuilder {
            gizmos: self,
            position,
            rotation: Mat2::from_angle(angle),
            half_size,
            color: color.into(),
            segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }

    /// Draw a circle in 3D at `position` with the flat side facing `normal`.
    ///
    /// This should be called for each frame the circle needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.circle(Vec3::ZERO, Dir3::Z, 1., GREEN);
    ///
    ///     // Circles have 32 line-segments by default.
    ///     // You may want to increase this for larger circles.
    ///     gizmos
    ///         .circle(Vec3::ZERO, Dir3::Z, 5., RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn circle(
        &mut self,
        position: Vec3,
        normal: Dir3,
        radius: f32,
        color: impl Into<Color>,
    ) -> EllipseBuilder<'_, 'w, 's, Config, Clear> {
        EllipseBuilder {
            gizmos: self,
            position,
            rotation: Quat::from_rotation_arc(Vec3::Z, *normal),
            half_size: Vec2::splat(radius),
            color: color.into(),
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
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.circle_2d(Vec2::ZERO, 1., GREEN);
    ///
    ///     // Circles have 32 line-segments by default.
    ///     // You may want to increase this for larger circles.
    ///     gizmos
    ///         .circle_2d(Vec2::ZERO, 5., RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn circle_2d(
        &mut self,
        position: Vec2,
        radius: f32,
        color: impl Into<Color>,
    ) -> Ellipse2dBuilder<'_, 'w, 's, Config, Clear> {
        Ellipse2dBuilder {
            gizmos: self,
            position,
            rotation: Mat2::IDENTITY,
            half_size: Vec2::splat(radius),
            color: color.into(),
            segments: DEFAULT_CIRCLE_SEGMENTS,
        }
    }
}

/// A builder returned by [`Gizmos::ellipse`].
pub struct EllipseBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,
    position: Vec3,
    rotation: Quat,
    half_size: Vec2,
    color: Color,
    segments: usize,
}

impl<Config, Clear> EllipseBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of line-segments for this ellipse.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<Config, Clear> Drop for EllipseBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let positions = ellipse_inner(self.half_size, self.segments)
            .map(|vec2| self.rotation * vec2.extend(0.))
            .map(|vec3| vec3 + self.position);
        self.gizmos.linestrip(positions, self.color);
    }
}

/// A builder returned by [`Gizmos::ellipse_2d`].
pub struct Ellipse2dBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,
    position: Vec2,
    rotation: Mat2,
    half_size: Vec2,
    color: Color,
    segments: usize,
}

impl<Config, Clear> Ellipse2dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of line-segments for this ellipse.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }
}

impl<Config, Clear> Drop for Ellipse2dBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of line-segments for this ellipse.
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        };

        let positions = ellipse_inner(self.half_size, self.segments)
            .map(|vec2| self.rotation * vec2)
            .map(|vec2| vec2 + self.position);
        self.gizmos.linestrip_2d(positions, self.color);
    }
}
