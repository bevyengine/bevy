//! Additional [`Gizmos`] Functions -- Arcs
//!
//! Includes the implementation of [`Gizmos::arc_2d`],
//! and assorted support items.

use crate::circles::DEFAULT_CIRCLE_SEGMENTS;
use crate::prelude::Gizmos;
use bevy_math::Vec2;
use bevy_render::color::Color;
use std::f32::consts::TAU;

impl<'s> Gizmos<'s> {
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
    ) -> Arc2dBuilder<'_, 's> {
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
pub struct Arc2dBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,
    position: Vec2,
    direction_angle: f32,
    arc_angle: f32,
    radius: f32,
    color: Color,
    segments: Option<usize>,
}

impl Arc2dBuilder<'_, '_> {
    /// Set the number of line-segments for this arc.
    pub fn segments(mut self, segments: usize) -> Self {
        self.segments = Some(segments);
        self
    }
}

impl Drop for Arc2dBuilder<'_, '_> {
    fn drop(&mut self) {
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
