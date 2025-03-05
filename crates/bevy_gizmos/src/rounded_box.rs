//! Additional [`GizmoBuffer`] Functions -- Rounded cuboids and rectangles
//!
//! Includes the implementation of [`GizmoBuffer::rounded_rect`], [`GizmoBuffer::rounded_rect_2d`] and [`GizmoBuffer::rounded_cuboid`].
//! and assorted support items.

use core::f32::consts::FRAC_PI_2;

use crate::{gizmos::GizmoBuffer, prelude::GizmoConfigGroup};
use bevy_color::Color;
use bevy_math::{Isometry2d, Isometry3d, Quat, Vec2, Vec3};
use bevy_transform::components::Transform;

/// A builder returned by [`GizmoBuffer::rounded_rect`] and [`GizmoBuffer::rounded_rect_2d`]
pub struct RoundedRectBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    size: Vec2,
    gizmos: &'a mut GizmoBuffer<Config, Clear>,
    config: RoundedBoxConfig,
}
/// A builder returned by [`GizmoBuffer::rounded_cuboid`]
pub struct RoundedCuboidBuilder<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    size: Vec3,
    gizmos: &'a mut GizmoBuffer<Config, Clear>,
    config: RoundedBoxConfig,
}
struct RoundedBoxConfig {
    isometry: Isometry3d,
    color: Color,
    corner_radius: f32,
    arc_resolution: u32,
}

impl<Config, Clear> RoundedRectBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Change the radius of the corners to be `corner_radius`.
    /// The default corner radius is [min axis of size] / 10.0
    pub fn corner_radius(mut self, corner_radius: f32) -> Self {
        self.config.corner_radius = corner_radius;
        self
    }

    /// Change the resolution of the arcs at the corners of the rectangle.
    /// The default value is 8
    pub fn arc_resolution(mut self, arc_resolution: u32) -> Self {
        self.config.arc_resolution = arc_resolution;
        self
    }
}

impl<Config, Clear> RoundedCuboidBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Change the radius of the edges to be `edge_radius`.
    /// The default edge radius is [min axis of size] / 10.0
    pub fn edge_radius(mut self, edge_radius: f32) -> Self {
        self.config.corner_radius = edge_radius;
        self
    }

    /// Change the resolution of the arcs at the edges of the cuboid.
    /// The default value is 8
    pub fn arc_resolution(mut self, arc_resolution: u32) -> Self {
        self.config.arc_resolution = arc_resolution;
        self
    }
}

impl<Config, Clear> Drop for RoundedRectBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }
        let config = &self.config;

        // Calculate inner and outer half size and ensure that the edge_radius is <= any half_length
        let mut outer_half_size = self.size.abs() / 2.0;
        let inner_half_size =
            (outer_half_size - Vec2::splat(config.corner_radius.abs())).max(Vec2::ZERO);
        let corner_radius = (outer_half_size - inner_half_size).min_element();
        let mut inner_half_size = outer_half_size - Vec2::splat(corner_radius);

        if config.corner_radius < 0. {
            core::mem::swap(&mut outer_half_size, &mut inner_half_size);
        }

        // Handle cases where the rectangle collapses into simpler shapes
        if outer_half_size.x * outer_half_size.y == 0. {
            self.gizmos.line(
                config.isometry * -outer_half_size.extend(0.),
                config.isometry * outer_half_size.extend(0.),
                config.color,
            );
            return;
        }
        if corner_radius == 0. {
            self.gizmos.rect(config.isometry, self.size, config.color);
            return;
        }

        let vertices = [
            // top right
            Vec3::new(inner_half_size.x, outer_half_size.y, 0.),
            Vec3::new(inner_half_size.x, inner_half_size.y, 0.),
            Vec3::new(outer_half_size.x, inner_half_size.y, 0.),
            // bottom right
            Vec3::new(outer_half_size.x, -inner_half_size.y, 0.),
            Vec3::new(inner_half_size.x, -inner_half_size.y, 0.),
            Vec3::new(inner_half_size.x, -outer_half_size.y, 0.),
            // bottom left
            Vec3::new(-inner_half_size.x, -outer_half_size.y, 0.),
            Vec3::new(-inner_half_size.x, -inner_half_size.y, 0.),
            Vec3::new(-outer_half_size.x, -inner_half_size.y, 0.),
            // top left
            Vec3::new(-outer_half_size.x, inner_half_size.y, 0.),
            Vec3::new(-inner_half_size.x, inner_half_size.y, 0.),
            Vec3::new(-inner_half_size.x, outer_half_size.y, 0.),
        ]
        .map(|vec3| config.isometry * vec3);

        for chunk in vertices.chunks_exact(3) {
            self.gizmos
                .short_arc_3d_between(chunk[1], chunk[0], chunk[2], config.color)
                .resolution(config.arc_resolution);
        }

        let edges = if config.corner_radius > 0. {
            [
                (vertices[2], vertices[3]),
                (vertices[5], vertices[6]),
                (vertices[8], vertices[9]),
                (vertices[11], vertices[0]),
            ]
        } else {
            [
                (vertices[0], vertices[5]),
                (vertices[3], vertices[8]),
                (vertices[6], vertices[11]),
                (vertices[9], vertices[2]),
            ]
        };

        for (start, end) in edges {
            self.gizmos.line(start, end, config.color);
        }
    }
}

impl<Config, Clear> Drop for RoundedCuboidBuilder<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }
        let config = &self.config;

        // Calculate inner and outer half size and ensure that the edge_radius is <= any half_length
        let outer_half_size = self.size.abs() / 2.0;
        let inner_half_size =
            (outer_half_size - Vec3::splat(config.corner_radius.abs())).max(Vec3::ZERO);
        let mut edge_radius = (outer_half_size - inner_half_size).min_element();
        let inner_half_size = outer_half_size - Vec3::splat(edge_radius);
        edge_radius *= config.corner_radius.signum();

        // Handle cases where the rounded cuboid collapses into simpler shapes
        if edge_radius == 0.0 {
            let transform = Transform::from_translation(config.isometry.translation.into())
                .with_rotation(config.isometry.rotation)
                .with_scale(self.size);
            self.gizmos.cuboid(transform, config.color);
            return;
        }

        let rects = [
            (
                Vec3::X,
                Vec2::new(self.size.z, self.size.y),
                Quat::from_rotation_y(FRAC_PI_2),
            ),
            (
                Vec3::Y,
                Vec2::new(self.size.x, self.size.z),
                Quat::from_rotation_x(FRAC_PI_2),
            ),
            (Vec3::Z, Vec2::new(self.size.x, self.size.y), Quat::IDENTITY),
        ];

        for (position, size, rotation) in rects {
            let local_position = position * inner_half_size;
            self.gizmos
                .rounded_rect(
                    config.isometry * Isometry3d::new(local_position, rotation),
                    size,
                    config.color,
                )
                .arc_resolution(config.arc_resolution)
                .corner_radius(edge_radius);

            self.gizmos
                .rounded_rect(
                    config.isometry * Isometry3d::new(-local_position, rotation),
                    size,
                    config.color,
                )
                .arc_resolution(config.arc_resolution)
                .corner_radius(edge_radius);
        }
    }
}

impl<Config, Clear> GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draw a wireframe rectangle with rounded corners in 3D.
    ///
    /// This should be called for each frame the rectangle needs to be rendered.
    ///
    /// # Arguments
    ///
    /// - `isometry` defines the translation and rotation of the rectangle.
    ///   - the translation specifies the center of the rectangle
    ///   - defines orientation of the rectangle, by default we assume the rectangle is contained in
    ///     a plane parallel to the XY plane.
    /// - `size`: defines the size of the rectangle. This refers to the 'outer size', similar to a bounding box.
    /// - `color`: color of the rectangle
    ///
    /// # Builder methods
    ///
    /// - The corner radius can be adjusted with the `.corner_radius(...)` method.
    /// - The resolution of the arcs at each corner (i.e. the level of detail) can be adjusted with the
    ///   `.arc_resolution(...)` method.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::css::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rounded_rect(
    ///         Isometry3d::IDENTITY,
    ///         Vec2::ONE,
    ///         GREEN
    ///         )
    ///         .corner_radius(0.25)
    ///         .arc_resolution(10);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn rounded_rect(
        &mut self,
        isometry: impl Into<Isometry3d>,
        size: Vec2,
        color: impl Into<Color>,
    ) -> RoundedRectBuilder<'_, Config, Clear> {
        let corner_radius = size.min_element() * DEFAULT_CORNER_RADIUS;
        RoundedRectBuilder {
            gizmos: self,
            config: RoundedBoxConfig {
                isometry: isometry.into(),
                color: color.into(),
                corner_radius,
                arc_resolution: DEFAULT_ARC_RESOLUTION,
            },
            size,
        }
    }

    /// Draw a wireframe rectangle with rounded corners in 2D.
    ///
    /// This should be called for each frame the rectangle needs to be rendered.
    ///
    /// # Arguments
    ///
    /// - `isometry` defines the translation and rotation of the rectangle.
    ///   - the translation specifies the center of the rectangle
    ///   - defines orientation of the rectangle, by default we assume the rectangle aligned with all axes.
    /// - `size`: defines the size of the rectangle. This refers to the 'outer size', similar to a bounding box.
    /// - `color`: color of the rectangle
    ///
    /// # Builder methods
    ///
    /// - The corner radius can be adjusted with the `.corner_radius(...)` method.
    /// - The resolution of the arcs at each corner (i.e. the level of detail) can be adjusted with the
    ///   `.arc_resolution(...)` method.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::css::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rounded_rect_2d(
    ///         Isometry2d::IDENTITY,
    ///         Vec2::ONE,
    ///         GREEN
    ///         )
    ///         .corner_radius(0.25)
    ///         .arc_resolution(10);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn rounded_rect_2d(
        &mut self,
        isometry: impl Into<Isometry2d>,
        size: Vec2,
        color: impl Into<Color>,
    ) -> RoundedRectBuilder<'_, Config, Clear> {
        let isometry = isometry.into();
        let corner_radius = size.min_element() * DEFAULT_CORNER_RADIUS;
        RoundedRectBuilder {
            gizmos: self,
            config: RoundedBoxConfig {
                isometry: Isometry3d::new(
                    isometry.translation.extend(0.0),
                    Quat::from_rotation_z(isometry.rotation.as_radians()),
                ),
                color: color.into(),
                corner_radius,
                arc_resolution: DEFAULT_ARC_RESOLUTION,
            },
            size,
        }
    }

    /// Draw a wireframe cuboid with rounded corners in 3D.
    ///
    /// This should be called for each frame the cuboid needs to be rendered.
    ///
    /// # Arguments
    ///
    /// - `isometry` defines the translation and rotation of the cuboid.
    ///   - the translation specifies the center of the cuboid
    ///   - defines orientation of the cuboid, by default we assume the cuboid aligned with all axes.
    /// - `size`: defines the size of the cuboid. This refers to the 'outer size', similar to a bounding box.
    /// - `color`: color of the cuboid
    ///
    /// # Builder methods
    ///
    /// - The edge radius can be adjusted with the `.edge_radius(...)` method.
    /// - The resolution of the arcs at each edge (i.e. the level of detail) can be adjusted with the
    ///   `.arc_resolution(...)` method.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::css::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rounded_cuboid(
    ///         Isometry3d::IDENTITY,
    ///         Vec3::ONE,
    ///         GREEN
    ///         )
    ///         .edge_radius(0.25)
    ///         .arc_resolution(10);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn rounded_cuboid(
        &mut self,
        isometry: impl Into<Isometry3d>,
        size: Vec3,
        color: impl Into<Color>,
    ) -> RoundedCuboidBuilder<'_, Config, Clear> {
        let corner_radius = size.min_element() * DEFAULT_CORNER_RADIUS;
        RoundedCuboidBuilder {
            gizmos: self,
            config: RoundedBoxConfig {
                isometry: isometry.into(),
                color: color.into(),
                corner_radius,
                arc_resolution: DEFAULT_ARC_RESOLUTION,
            },
            size,
        }
    }
}

const DEFAULT_ARC_RESOLUTION: u32 = 8;
const DEFAULT_CORNER_RADIUS: f32 = 0.1;
