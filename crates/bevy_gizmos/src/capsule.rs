//! Additional [`GizmoBuffer`] Functions -- Capsule.
//!
//! Includes the implementation of [`GizmoBuffer::capsule`], [`GizmoBuffer::hemisphere`], and assorted support items.

use crate::{circles::DEFAULT_CIRCLE_RESOLUTION, gizmos::GizmoBuffer, prelude::GizmoConfigGroup};
use bevy_color::Color;
use bevy_math::{ops::sin_cos, Isometry3d, Quat, Vec2, Vec3, Vec3Swizzles};
use core::f32::consts::{FRAC_PI_2, PI, TAU};

/// Calculates half of an ellipse.
fn half_ellipse(half_size: Vec2, resolution: u32) -> impl Iterator<Item = Vec2> {
    (0..resolution + 1).map(move |i| {
        let angle = i as f32 * PI / resolution as f32;
        let (x, y) = sin_cos(angle - FRAC_PI_2);
        Vec2::new(x, y) * half_size
    })
}

impl<Config, Clear> GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draw a wireframe capsule in 3D made of 2 hemispheres connected with lines with the given `isometry` applied.
    ///
    /// If `isometry == Isometry3d::IDENTITY` then
    ///
    /// - the center is at `Vec3::ZERO`,
    /// - the length is aligned with the `Vec3::X` axes
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.capsule(Isometry3d::IDENTITY, 0.5, 0.5, GREEN);
    ///
    ///     // Capsules have a 32 line-segment resolution by default.
    ///     // You may want to increase this for larger capsules.
    ///     gizmos
    ///         .capsule(Isometry3d::IDENTITY, 0.5, 0.5, RED)
    ///         .resolution(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn capsule(
        &mut self,
        isometry: impl Into<Isometry3d>,
        radius: f32,
        length: f32,
        color: impl Into<Color>,
    ) -> CapsuleBuilder<'_, Config, Clear> {
        CapsuleBuilder {
            gizmos: self,
            isometry: isometry.into(),
            radius,
            length,
            color: color.into(),
            resolution: DEFAULT_CIRCLE_RESOLUTION,
        }
    }
}

/// A builder returned by [`GizmoBuffer::capsule`].
pub struct CapsuleBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,
    isometry: Isometry3d,

    // Radius of the capsule
    radius: f32,

    // Length of the capsule minus the hemispherical caps
    length: f32,

    // Color of the capsule
    color: Color,

    // Number of line-segments used to approximate the capsule geometry
    resolution: u32,
}

impl<Config, Clear> CapsuleBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of line-segments used to approximate the capsule geometry.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl<Config, Clear> Drop for CapsuleBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }
        let half_length = self.length / 2.0;

        // Offset and rotation of the bottom hemisphere.
        let base_cap = Isometry3d::new(
            Vec3::new(-half_length, 0., 0.),
            Quat::from_rotation_z(FRAC_PI_2),
        );
        self.gizmos.hemisphere(
            self.isometry * base_cap,
            Vec2::new(self.radius, self.radius),
            self.color,
        );

        // Offset and rotation of the top hemisphere.
        let top_cap = Isometry3d::new(
            Vec3::new(half_length, 0., 0.),
            Quat::from_rotation_z(-FRAC_PI_2),
        );
        self.gizmos.hemisphere(
            self.isometry * top_cap,
            Vec2::new(self.radius, self.radius),
            self.color,
        );

        // Connect the hemispheres together with 4 lines to comprise the body.
        let step_theta = TAU / 4.0;
        for pos in 0..4 {
            let theta = pos as f32 * step_theta;
            let (sin, cos) = sin_cos(theta);
            let start = self.isometry
                * Isometry3d::from_translation(Vec3::new(
                    -half_length,
                    self.radius * cos,
                    self.radius * sin,
                ));
            let end = self.isometry
                * Isometry3d::from_translation(Vec3::new(
                    half_length,
                    self.radius * cos,
                    self.radius * sin,
                ));
            self.gizmos
                .line(start.translation.into(), end.translation.into(), self.color);
        }
    }
}

impl<Config, Clear> GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draw an wireframe hemiellipsoid in 3D made of an ellipse and 2 half-ellipses with the given `isometry` applied.
    ///
    /// If `isometry == Isometry3d::IDENTITY` then
    ///
    /// - the center is at `Vec3::ZERO`,
    /// - the apex is aligned with the `Vec3::Y` axes
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.hemisphere(Isometry3d::IDENTITY, Vec2::new(0.5, 0.5), GREEN);
    ///
    ///     // Hemispherses have a resolution of 32 line-segments by default.
    ///     // You may want to increase this for larger hemispheres.
    ///     gizmos
    ///         .hemisphere(Isometry3d::IDENTITY, Vec2::new(0.5, 0.5), RED)
    ///         .resolution(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn hemisphere(
        &mut self,
        isometry: impl Into<Isometry3d>,
        half_size: Vec2,
        color: impl Into<Color>,
    ) -> HemisphereBuilder<'_, Config, Clear> {
        HemisphereBuilder {
            gizmos: self,
            isometry: isometry.into(),
            half_size,
            color: color.into(),
            resolution: DEFAULT_CIRCLE_RESOLUTION,
        }
    }
}

/// A builder returned by [`GizmoBuffer::hemisphere`].
pub struct HemisphereBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,
    isometry: Isometry3d,

    // Size of the hemisphere along the X and Z axis
    half_size: Vec2,

    // Color of the hemisphere
    color: Color,

    // Number of line-segments used to approximate the hemisphere geometry
    resolution: u32,
}

impl<Config, Clear> HemisphereBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of line-segments used to approximate the hemisphere geometry.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl<Config, Clear> Drop for HemisphereBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        // The base circumference
        self.gizmos
            .ellipse(
                self.isometry
                    * Isometry3d::from_rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::Z)),
                self.half_size,
                self.color,
            )
            .resolution(self.resolution);

        // The height at which both the half-ellipses will meet
        let apex_height = self.half_size.min_element();

        // The half-ellipse along the X-axis
        let positions = half_ellipse(Vec2::new(self.half_size.x, apex_height), self.resolution)
            .map(|vec2| self.isometry * vec2.extend(0.));
        self.gizmos.linestrip(positions, self.color);

        // The half-ellipse along the Z-axes
        let positions = half_ellipse(Vec2::new(self.half_size.y, apex_height), self.resolution)
            .map(|vec2| self.isometry * Vec3::new(0., vec2.y, vec2.x));
        self.gizmos.linestrip(positions, self.color);
    }
}

impl<Config, Clear> GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draw a wireframe ellipsoid in 3D made out of 3 ellipses around each axes with the given
    /// `isometry` applied.
    ///
    /// If `isometry == Isometry3d::IDENTITY` then
    ///
    /// - the center is at `Vec3::ZERO`,
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::{RED, GREEN};
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.ellipsoid(Isometry3d::IDENTITY, Vec3::new(1.0, 0.5, 0.75), GREEN);
    ///
    ///     // Ellipsoids have a resolution of 32 line-segments by default.
    ///     // You may want to increase this for larger ellipsoids.
    ///     gizmos
    ///         .ellipsoid(Isometry3d::IDENTITY, Vec3::new(1.0, 0.5, 0.75), RED)
    ///         .resolution(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn ellipsoid(
        &mut self,
        isometry: impl Into<Isometry3d>,
        half_extents: Vec3,
        color: impl Into<Color>,
    ) -> EllipsoidBuilder<'_, Config, Clear> {
        EllipsoidBuilder {
            gizmos: self,
            isometry: isometry.into(),
            half_extents,
            color: color.into(),
            resolution: DEFAULT_CIRCLE_RESOLUTION,
        }
    }
}

/// A builder returned by [`GizmoBuffer::ellipsoid`].
pub struct EllipsoidBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,
    isometry: Isometry3d,

    // Radius of the ellipsoid along each axis
    half_extents: Vec3,

    // Color of the ellipsoid
    color: Color,

    // Number of line-segments used to approximate the ellipsoid geometry
    resolution: u32,
}

impl<Config, Clear> EllipsoidBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Set the number of line-segments used to approximate the ellipsoid geometry.
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }
}

impl<Config, Clear> Drop for EllipsoidBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        self.gizmos
            .ellipse(self.isometry, self.half_extents.xy(), self.color)
            .resolution(self.resolution);

        self.gizmos
            .ellipse(
                self.isometry
                    * Isometry3d::from_rotation(Quat::from_rotation_arc(Vec3::Y, Vec3::Z)),
                self.half_extents.xz(),
                self.color,
            )
            .resolution(self.resolution);

        self.gizmos
            .ellipse(
                self.isometry
                    * Isometry3d::from_rotation(Quat::from_rotation_arc(Vec3::X, Vec3::Z)),
                self.half_extents.zy(),
                self.color,
            )
            .resolution(self.resolution);
    }
}
