//! Additional [`Gizmos`] Functions -- Rounded cuboids and rectangles
//!
//! Includes the implementation of [`Gizmos::rounded_rect`], [`Gizmos::rounded_rect_2d`] and [`Gizmos::rounded_cuboid`].
//! and assorted support items.

use std::f32::consts::FRAC_PI_2;

use crate::prelude::{GizmoConfigGroup, Gizmos};
use bevy_color::Color;
use bevy_math::{Quat, Vec2, Vec3};
use bevy_transform::components::Transform;

/// A builder returned by [`Gizmos::rounded_rect`] and [`Gizmos::rounded_rect_2d`]
pub struct RoundedRectBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    size: Vec2,
    gizmos: &'a mut Gizmos<'w, 's, T>,
    config: RoundedBoxConfig,
}
/// A builder returned by [`Gizmos::rounded_cuboid`]
pub struct RoundedCuboidBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    size: Vec3,
    gizmos: &'a mut Gizmos<'w, 's, T>,
    config: RoundedBoxConfig,
}
struct RoundedBoxConfig {
    position: Vec3,
    rotation: Quat,
    color: Color,
    corner_radius: f32,
    arc_resolution: u32,
}

impl<T: GizmoConfigGroup> RoundedRectBuilder<'_, '_, '_, T> {
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
impl<T: GizmoConfigGroup> RoundedCuboidBuilder<'_, '_, '_, T> {
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

impl<T: GizmoConfigGroup> Drop for RoundedRectBuilder<'_, '_, '_, T> {
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
            std::mem::swap(&mut outer_half_size, &mut inner_half_size);
        }

        // Handle cases where the rectangle collapses into simpler shapes
        if outer_half_size.x * outer_half_size.y == 0. {
            self.gizmos.line(
                config.position + config.rotation * -outer_half_size.extend(0.),
                config.position + config.rotation * outer_half_size.extend(0.),
                config.color,
            );
            return;
        }
        if corner_radius == 0. {
            self.gizmos
                .rect(config.position, config.rotation, self.size, config.color);
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
        .map(|v| config.position + config.rotation * v);

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

impl<T: GizmoConfigGroup> Drop for RoundedCuboidBuilder<'_, '_, '_, T> {
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
            let transform = Transform::from_translation(config.position)
                .with_rotation(config.rotation)
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
            let world_rotation = config.rotation * rotation;
            let local_position = config.rotation * (position * inner_half_size);
            self.gizmos
                .rounded_rect(
                    config.position + local_position,
                    world_rotation,
                    size,
                    config.color,
                )
                .arc_resolution(config.arc_resolution)
                .corner_radius(edge_radius);

            self.gizmos
                .rounded_rect(
                    config.position - local_position,
                    world_rotation,
                    size,
                    config.color,
                )
                .arc_resolution(config.arc_resolution)
                .corner_radius(edge_radius);
        }
    }
}

impl<'w, 's, T: GizmoConfigGroup> Gizmos<'w, 's, T> {
    /// Draw a wireframe rectangle with rounded corners in 3D.
    ///
    /// This should be called for each frame the rectangle needs to be rendered.
    ///
    /// # Arguments
    ///
    /// - `position`: The center point of the rectangle.
    /// - `rotation`: defines orientation of the rectangle, by default we assume the rectangle is contained in a plane parallel to the XY plane.
    /// - `size`: defines the size of the rectangle. This refers to the 'outer size', similar to a bounding box.
    /// - `color`: color of the rectangle
    ///
    /// # Builder methods
    ///
    /// - The corner radius can be adjusted with the `.corner_radius(...)` method.
    /// - The resolution of the arcs at each corner (i.e. the level of detail) can be adjusted with the
    ///     `.arc_resolution(...)` method.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::css::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rounded_rect(
    ///         Vec3::ZERO,
    ///         Quat::IDENTITY,
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
        position: Vec3,
        rotation: Quat,
        size: Vec2,
        color: impl Into<Color>,
    ) -> RoundedRectBuilder<'_, 'w, 's, T> {
        let corner_radius = size.min_element() * DEFAULT_CORNER_RADIUS;
        RoundedRectBuilder {
            gizmos: self,
            config: RoundedBoxConfig {
                position,
                rotation,
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
    /// - `position`: The center point of the rectangle.
    /// - `rotation`: defines orientation of the rectangle.
    /// - `size`: defines the size of the rectangle. This refers to the 'outer size', similar to a bounding box.
    /// - `color`: color of the rectangle
    ///
    /// # Builder methods
    ///
    /// - The corner radius can be adjusted with the `.corner_radius(...)` method.
    /// - The resolution of the arcs at each corner (i.e. the level of detail) can be adjusted with the
    ///     `.arc_resolution(...)` method.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::css::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rounded_rect_2d(
    ///         Vec2::ZERO,
    ///         0.,
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
        position: Vec2,
        rotation: f32,
        size: Vec2,
        color: impl Into<Color>,
    ) -> RoundedRectBuilder<'_, 'w, 's, T> {
        let corner_radius = size.min_element() * DEFAULT_CORNER_RADIUS;
        RoundedRectBuilder {
            gizmos: self,
            config: RoundedBoxConfig {
                position: position.extend(0.),
                rotation: Quat::from_rotation_z(rotation),
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
    /// - `position`: The center point of the cuboid.
    /// - `rotation`: defines orientation of the cuboid.
    /// - `size`: defines the size of the cuboid. This refers to the 'outer size', similar to a bounding box.
    /// - `color`: color of the cuboid
    ///
    /// # Builder methods
    ///
    /// - The edge radius can be adjusted with the `.edge_radius(...)` method.
    /// - The resolution of the arcs at each edge (i.e. the level of detail) can be adjusted with the
    ///     `.arc_resolution(...)` method.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::css::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.rounded_cuboid(
    ///         Vec3::ZERO,
    ///         Quat::IDENTITY,
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
        position: Vec3,
        rotation: Quat,
        size: Vec3,
        color: impl Into<Color>,
    ) -> RoundedCuboidBuilder<'_, 'w, 's, T> {
        let corner_radius = size.min_element() * DEFAULT_CORNER_RADIUS;
        RoundedCuboidBuilder {
            gizmos: self,
            config: RoundedBoxConfig {
                position,
                rotation,
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
