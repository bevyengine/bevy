//! Additional [`Gizmos`] Functions -- Circles
//!
//! Includes the implementation of [`Gizmos::circle`] and [`Gizmos::circle_2d`],
//! and assorted support items.

use crate::prelude::{GizmoConfigGroup, Gizmos};
use bevy_color::Color;
use bevy_math::Mat2;
use bevy_math::{Dir3, Quat, Vec2, Vec3};
use std::f32::consts::TAU;

pub(crate) const DEFAULT_CIRCLE_RESOLUTION: u32 = 32;

fn ellipse_inner(half_size: Vec2, resolution: u32) -> impl Iterator<Item = Vec2> {
    (0..resolution + 1).map(move |i| {
        let angle = i as f32 * TAU / resolution as f32;
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
    ///         .resolution(64);
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
            resolution: DEFAULT_CIRCLE_RESOLUTION,
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
    ///         .resolution(64);
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
            resolution: DEFAULT_CIRCLE_RESOLUTION,
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
    ///         .resolution(64);
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
            resolution: DEFAULT_CIRCLE_RESOLUTION,
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
    ///         .resolution(64);
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
            resolution: DEFAULT_CIRCLE_RESOLUTION,
        }
    }

    /// Draw a wireframe sphere in 3D made out of 3 circles around the axes.
    ///
    /// This should be called for each frame the sphere needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::Color;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.sphere(Vec3::ZERO, Quat::IDENTITY, 1., Color::BLACK);
    ///
    ///     // Each circle has 32 line-segments by default.
    ///     // You may want to increase this for larger spheres.
    ///     gizmos
    ///         .sphere(Vec3::ZERO, Quat::IDENTITY, 5., Color::BLACK)
    ///         .resolution(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn sphere(
        &mut self,
        position: Vec3,
        rotation: Quat,
        radius: f32,
        color: impl Into<Color>,
    ) -> SphereBuilder<'_, 'w, 's, Config, Clear> {
        SphereBuilder {
            gizmos: self,
            radius,
            position,
            rotation,
            color: color.into(),
            resolution: DEFAULT_CIRCLE_RESOLUTION,
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
    resolution: u32,
}

impl<Config, Clear> EllipseBuilder<'_, '_, '_, Config, Clear>
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

impl<Config, Clear> Drop for EllipseBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let positions = ellipse_inner(self.half_size, self.resolution)
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
    resolution: u32,
}

impl<Config, Clear> Ellipse2dBuilder<'_, '_, '_, Config, Clear>
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

        let positions = ellipse_inner(self.half_size, self.resolution)
            .map(|vec2| self.rotation * vec2)
            .map(|vec2| vec2 + self.position);
        self.gizmos.linestrip_2d(positions, self.color);
    }
}

/// Builder for configuring the drawing options of [`Sphere`].
pub struct SphereBuilder<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,

    // Radius of the sphere
    radius: f32,

    // Rotation of the sphere around the origin in 3D space
    rotation: Quat,
    // Center position of the sphere in 3D space
    position: Vec3,
    // Color of the sphere
    color: Color,

    // Number of line-segments used to approximate the sphere geometry
    resolution: u32,
}

impl<Config, Clear> SphereBuilder<'_, '_, '_, Config, Clear>
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

impl<Config, Clear> Drop for SphereBuilder<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        let SphereBuilder {
            radius,
            position: center,
            rotation,
            color,
            resolution,
            ..
        } = self;

        // draws one great circle around each of the local axes
        Vec3::AXES.into_iter().for_each(|axis| {
            let normal = *rotation * axis;
            self.gizmos
                .circle(*center, Dir3::new_unchecked(normal), *radius, *color)
                .resolution(*resolution);
        });
    }
}
