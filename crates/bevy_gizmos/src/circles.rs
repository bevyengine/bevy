//! Additional [`GizmoBuffer`] Functions -- Circles
//!
//! Includes the implementation of [`GizmoBuffer::circle`] and [`GizmoBuffer::circle_2d`],
//! and assorted support items.

use crate::{gizmos::GizmoBuffer, prelude::GizmoConfigGroup};
use bevy_color::Color;
use bevy_math::{ops, Isometry2d, Isometry3d, Quat, Vec2, Vec3};
use core::f32::consts::TAU;

pub(crate) const DEFAULT_CIRCLE_RESOLUTION: u32 = 32;

fn ellipse_inner(half_size: Vec2, resolution: u32) -> impl Iterator<Item = Vec2> {
    (0..resolution + 1).map(move |i| {
        let angle = i as f32 * TAU / resolution as f32;
        let (x, y) = ops::sin_cos(angle);
        Vec2::new(x, y) * half_size
    })
}

impl<Config, Clear> GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draw an ellipse in 3D with the given `isometry` applied.
    ///
    /// If `isometry == Isometry3d::IDENTITY` then
    ///
    /// - the center is at `Vec3::ZERO`
    /// - the `half_sizes` are aligned with the `Vec3::X` and `Vec3::Y` axes.
    ///
    /// This should be called for each frame the ellipse needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ellipse(Isometry3d::IDENTITY, Vec2::new(1., 2.), GREEN);
    ///
    ///     // Ellipses have 32 line-segments by default.
    ///     // You may want to increase this for larger ellipses.
    ///     gizmos
    ///         .ellipse(Isometry3d::IDENTITY, Vec2::new(5., 1.), RED)
    ///         .resolution(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ellipse(
        &mut self,
        isometry: impl Into<Isometry3d>,
        half_size: Vec2,
        color: impl Into<Color>,
    ) -> EllipseBuilder<'_, Config, Clear> {
        EllipseBuilder {
            gizmos: self,
            isometry: isometry.into(),
            half_size,
            color: color.into(),
            resolution: DEFAULT_CIRCLE_RESOLUTION,
        }
    }

    /// Draw an ellipse in 2D with the given `isometry` applied.
    ///
    /// If `isometry == Isometry2d::IDENTITY` then
    ///
    /// - the center is at `Vec2::ZERO`
    /// - the `half_sizes` are aligned with the `Vec2::X` and `Vec2::Y` axes.
    ///
    /// This should be called for each frame the ellipse needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ellipse_2d(Isometry2d::from_rotation(Rot2::degrees(180.0)), Vec2::new(2., 1.), GREEN);
    ///
    ///     // Ellipses have 32 line-segments by default.
    ///     // You may want to increase this for larger ellipses.
    ///     gizmos
    ///         .ellipse_2d(Isometry2d::from_rotation(Rot2::degrees(180.0)), Vec2::new(5., 1.), RED)
    ///         .resolution(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ellipse_2d(
        &mut self,
        isometry: impl Into<Isometry2d>,
        half_size: Vec2,
        color: impl Into<Color>,
    ) -> Ellipse2dBuilder<'_, Config, Clear> {
        Ellipse2dBuilder {
            gizmos: self,
            isometry: isometry.into(),
            half_size,
            color: color.into(),
            resolution: DEFAULT_CIRCLE_RESOLUTION,
        }
    }

    /// Draw a circle in 3D with the given `isometry` applied.
    ///
    /// If `isometry == Isometry3d::IDENTITY` then
    ///
    /// - the center is at `Vec3::ZERO`
    /// - the radius is aligned with the `Vec3::X` and `Vec3::Y` axes.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.circle(Isometry3d::IDENTITY, 1., GREEN);
    ///
    ///     // Circles have 32 line-segments by default.
    ///     // You may want to increase this for larger circles.
    ///     gizmos
    ///         .circle(Isometry3d::IDENTITY, 5., RED)
    ///         .resolution(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn circle(
        &mut self,
        isometry: impl Into<Isometry3d>,
        radius: f32,
        color: impl Into<Color>,
    ) -> EllipseBuilder<'_, Config, Clear> {
        EllipseBuilder {
            gizmos: self,
            isometry: isometry.into(),
            half_size: Vec2::splat(radius),
            color: color.into(),
            resolution: DEFAULT_CIRCLE_RESOLUTION,
        }
    }

    /// Draw a circle in 2D with the given `isometry` applied.
    ///
    /// If `isometry == Isometry2d::IDENTITY` then
    ///
    /// - the center is at `Vec2::ZERO`
    /// - the radius is aligned with the `Vec2::X` and `Vec2::Y` axes.
    ///
    /// This should be called for each frame the circle needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.circle_2d(Isometry2d::IDENTITY, 1., GREEN);
    ///
    ///     // Circles have 32 line-segments by default.
    ///     // You may want to increase this for larger circles.
    ///     gizmos
    ///         .circle_2d(Isometry2d::IDENTITY, 5., RED)
    ///         .resolution(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn circle_2d(
        &mut self,
        isometry: impl Into<Isometry2d>,
        radius: f32,
        color: impl Into<Color>,
    ) -> Ellipse2dBuilder<'_, Config, Clear> {
        Ellipse2dBuilder {
            gizmos: self,
            isometry: isometry.into(),
            half_size: Vec2::splat(radius),
            color: color.into(),
            resolution: DEFAULT_CIRCLE_RESOLUTION,
        }
    }

    /// Draw a wireframe sphere in 3D made out of 3 circles around the axes with the given
    /// `isometry` applied.
    ///
    /// If `isometry == Isometry3d::IDENTITY` then
    ///
    /// - the center is at `Vec3::ZERO`
    /// - the 3 circles are in the XY, YZ and XZ planes.
    ///
    /// This should be called for each frame the sphere needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::Color;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.sphere(Isometry3d::IDENTITY, 1., Color::BLACK);
    ///
    ///     // Each circle has 32 line-segments by default.
    ///     // You may want to increase this for larger spheres.
    ///     gizmos
    ///         .sphere(Isometry3d::IDENTITY, 5., Color::BLACK)
    ///         .resolution(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn sphere(
        &mut self,
        isometry: impl Into<Isometry3d>,
        radius: f32,
        color: impl Into<Color>,
    ) -> SphereBuilder<'_, Config, Clear> {
        SphereBuilder {
            gizmos: self,
            radius,
            isometry: isometry.into(),
            color: color.into(),
            resolution: DEFAULT_CIRCLE_RESOLUTION,
        }
    }
}

/// A builder returned by [`GizmoBuffer::ellipse`].
pub struct EllipseBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,
    isometry: Isometry3d,
    half_size: Vec2,
    color: Color,
    resolution: u32,
}

impl<Config, Clear> EllipseBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of lines used to approximate the geometry of this ellipse.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl<Config, Clear> Drop for EllipseBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let positions = ellipse_inner(self.half_size, self.resolution)
            .map(|vec2| self.isometry * vec2.extend(0.));
        self.gizmos.linestrip(positions, self.color);
    }
}

/// A builder returned by [`GizmoBuffer::ellipse_2d`].
pub struct Ellipse2dBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,
    isometry: Isometry2d,
    half_size: Vec2,
    color: Color,
    resolution: u32,
}

impl<Config, Clear> Ellipse2dBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of line-segments used to approximate the geometry of this ellipse.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl<Config, Clear> Drop for Ellipse2dBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of line-segments for this ellipse.
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        };

        let positions =
            ellipse_inner(self.half_size, self.resolution).map(|vec2| self.isometry * vec2);
        self.gizmos.linestrip_2d(positions, self.color);
    }
}

/// A builder returned by [`GizmoBuffer::sphere`].
pub struct SphereBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,

    // Radius of the sphere
    radius: f32,

    isometry: Isometry3d,
    // Color of the sphere
    color: Color,

    // Number of line-segments used to approximate the sphere geometry
    resolution: u32,
}

impl<Config, Clear> SphereBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of line-segments used to approximate the sphere geometry.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl<Config, Clear> Drop for SphereBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        // draws one great circle around each of the local axes
        Vec3::AXES.into_iter().for_each(|axis| {
            let axis_rotation = Isometry3d::from_rotation(Quat::from_rotation_arc(Vec3::Z, axis));
            self.gizmos
                .circle(self.isometry * axis_rotation, self.radius, self.color)
                .resolution(self.resolution);
        });
    }
}
