//! Additional [`Gizmos`] Functions -- Arcs
//!
//! Includes the implementation of [`Gizmos::arc_2d`],
//! and assorted support items.

use crate::circles::DEFAULT_CIRCLE_SEGMENTS;
use crate::prelude::{GizmoConfigGroup, Gizmos};
use bevy_math::{Quat, Vec2, Vec3};
use bevy_render::color::Color;
use std::f32::consts::TAU;

// === 2D ===

impl<'w, 's, T: GizmoConfigGroup> Gizmos<'w, 's, T> {
    /// Draw an arc, which is a part of the circumference of a circle, in 2D.
    ///
    /// This should be called for each frame the arc needs to be rendered.
    ///
    /// # Arguments
    /// - `position` sets the center of this circle.
    /// - `radius` controls the distance from `position` to this arc, and thus its curvature.
    /// - `direction_angle` sets the clockwise  angle in radians between `Vec2::Y` and
    /// the vector from `position` to the midpoint of the arc.
    /// - `arc_angle` sets the length of this arc, in radians.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use std::f32::consts::PI;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.arc_2d(Vec2::ZERO, 0., PI / 4., 1., Color::GREEN);
    ///
    ///     // Arcs have 32 line-segments by default.
    ///     // You may want to increase this for larger arcs.
    ///     gizmos
    ///         .arc_2d(Vec2::ZERO, 0., PI / 4., 5., Color::RED)
    ///         .segments(64);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn arc_2d(
        &mut self,
        position: Vec2,
        direction_angle: f32,
        arc_angle: f32,
        radius: f32,
        color: Color,
    ) -> Arc2dBuilder<'_, 'w, 's, T> {
        Arc2dBuilder {
            gizmos: self,
            position,
            direction_angle,
            arc_angle,
            radius,
            color,
            segments: None,
        }
    }
}

/// A builder returned by [`Gizmos::arc_2d`].
pub struct Arc2dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,
    position: Vec2,
    direction_angle: f32,
    arc_angle: f32,
    radius: f32,
    color: Color,
    segments: Option<usize>,
}

impl<T: GizmoConfigGroup> Arc2dBuilder<'_, '_, '_, T> {
    /// Set the number of line-segments for this arc.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = Some(segments);
        self
    }
}

impl<T: GizmoConfigGroup> Drop for Arc2dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }
        let segments = match self.segments {
            Some(segments) => segments,
            // Do a linear interpolation between 1 and `DEFAULT_CIRCLE_SEGMENTS`
            // using the arc angle as scalar.
            None => ((self.arc_angle.abs() / TAU) * DEFAULT_CIRCLE_SEGMENTS as f32).ceil() as usize,
        };

        let positions = arc_inner(self.direction_angle, self.arc_angle, self.radius, segments)
            .map(|vec2| vec2 + self.position);
        self.gizmos.linestrip_2d(positions, self.color);
    }
}

fn arc_inner(
    direction_angle: f32,
    arc_angle: f32,
    radius: f32,
    segments: usize,
) -> impl Iterator<Item = Vec2> {
    (0..segments + 1).map(move |i| {
        let start = direction_angle - arc_angle / 2.;

        let angle = start + (i as f32 * (arc_angle / segments as f32));
        Vec2::from(angle.sin_cos()) * radius
    })
}

// === 3D ===

impl<'w, 's, T: GizmoConfigGroup> Gizmos<'w, 's, T> {
    /// Draw an arc, which is a part of the circumference of a circle, in 3D. This defaults to
    /// drawing a standard arc. This standard arc starts at `Vec3::X`, is embedded in the XZ plane, rotates counterclockwise and has the following default properties:
    ///
    /// - radius: 1.0
    /// - center: `Vec3::ZERO`
    /// - rotation: `Quat::IDENTITY` (in XZ plane, normal points upwards)
    /// - color: white
    /// - segments: depending on angle
    ///
    /// All of these properties can be modified with builder methods which are available on the
    /// returned struct.
    ///
    /// This should be called for each frame the arc needs to be rendered.
    ///
    /// # Arguments
    /// - `angle` sets how much of a circle circumference is passed, e.g. PI is half a circle. This
    /// value should be in the range (-2 * PI..=2 * PI)
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use std::f32::consts::PI;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.arc_2d(PI);
    ///
    ///     // This example shows how to modify the default settings
    ///
    ///     // rotation rotates normal to point in the direction of `Vec3::NEG_ONE`
    ///     let rotation = Quat::from_rotation_arc(Vec3::Y, Vec3::NEG_ONE.normalize())
    ///
    ///     gizmos
    ///        .arc_3d(270.0_f32.to_radians())
    ///        .radius(0.25)
    ///        .center(Vec3::ONE)
    ///        .rotation(rotation)
    ///        .segments(100)
    ///        .color(Color::ORANGE);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[inline]
    pub fn arc_3d(&mut self, angle: f32) -> Arc3dBuilder<'_, 'w, 's, T> {
        let segments = segments_from_angle(angle);
        Arc3dBuilder {
            gizmos: self,
            center: Vec3::ZERO,
            rotation: Quat::IDENTITY,
            angle,
            radius: 1.0,
            color: Color::default(),
            segments,
        }
    }
}

/// A builder returned by [`Gizmos::arc_2d`].
pub struct Arc3dBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,
    center: Vec3,
    rotation: Quat,
    angle: f32,
    radius: f32,
    color: Color,
    segments: usize,
}

impl<T: GizmoConfigGroup> Arc3dBuilder<'_, '_, '_, T> {
    /// Set the number of line-segments for this arc.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }

    /// Set the center of the standard arc
    pub fn center(mut self, center: Vec3) -> Self {
        self.center = center;
        self
    }

    /// Set the radius of the arc
    pub fn radius(mut self, radius: f32) -> Self {
        self.radius = radius;
        self
    }

    /// Rotate the standard arc from the XZ plane with this rotation
    pub fn rotation(mut self, rotation: Quat) -> Self {
        self.rotation = rotation;
        self
    }

    /// Set the color of the arc
    pub fn color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }
}

impl<T: GizmoConfigGroup> Drop for Arc3dBuilder<'_, '_, '_, T> {
    fn drop(&mut self) {
        let positions = arc3d_inner(
            self.center,
            self.rotation,
            self.angle,
            self.radius,
            self.segments,
        );
        self.gizmos
            .sphere(self.center, Quat::IDENTITY, 0.1, self.color);
        self.gizmos.linestrip(positions, self.color);
    }
}

fn arc3d_inner(
    center: Vec3,
    rotation: Quat,
    angle: f32,
    radius: f32,
    segments: usize,
) -> impl Iterator<Item = Vec3> {
    // drawing arcs bigger than TAU degrees or smaller than -TAU degrees makes no sense since
    // we won't see the overlap and we would just decrease the level of details since the segments
    // would be larger
    let angle = angle.clamp(-TAU, TAU);
    (0..=segments)
        .map(move |frac| frac as f32 / segments as f32)
        .map(move |percentage| angle * percentage)
        .map(move |frac_angle| Quat::from_axis_angle(Vec3::Y, frac_angle) * Vec3::X)
        .map(move |p| rotation * (p * radius) + center)
}

// helper function for getting a default value for the segments parameter
fn segments_from_angle(angle: f32) -> usize {
    ((angle.abs() / TAU) * DEFAULT_CIRCLE_SEGMENTS as f32).ceil() as usize
}
