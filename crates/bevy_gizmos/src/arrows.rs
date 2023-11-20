//! Additional [`Gizmos`] Functions -- Arrows
//!
//! Includes the implementation of [`Gizmos::arrow`] and [`Gizmos::arrow_2d`],
//! and assorted support items.

use crate::prelude::Gizmos;
use bevy_math::{Quat, Vec2, Vec3};
use bevy_render::color::Color;

/// A builder returned by [`Gizmos::arrow`] and [`Gizmos::arrow_2d`]
pub struct ArrowBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,
    start: Vec3,
    end: Vec3,
    color: Color,
    tip_length: f32,
    double_ended: bool,
}

/// Represents how the end of an arrow should be drawn.
/// See also [`Gizmos::arrow`] and [`Gizmos::arrow_2d`].
#[derive(Default, Debug)]
pub enum ArrowHead {
    /// No head. Putting this on both ends causes the arrow to just be a line.
    None,
    /// General-purpose arrow head with four tips for viewing from any angle. Default
    #[default]
    Normal,
    /// Two-tip arrow head, facing as close to `towards` as possible while still being inline with the arrow body
    Billboarded(
        /// Arrow will attempt to be most visible from this direction.
        /// - in 3d applications, this would typically be the camera position.
        /// - in 2d applications, this would typically be [`Vec3::Y`] or [`Vec3::Z`]
        Vec3,
    ),
}

impl ArrowBuilder<'_, '_> {
    /// Change the length of the tips to be `length`.
    /// The default tip length is [length of the arrow]/10.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.arrow(Vec3::ZERO, Vec3::ONE, Color::GREEN)
    ///         .with_tip_length(3.);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[doc(alias = "arrow_head_length")]
    pub fn with_tip_length(&mut self, length: f32) {
        self.tip_length = length;
    }

    /// Make the arrow double-ended.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.arrow(Vec3::ZERO, Vec3::ONE, Color::GREEN)
    ///         .with_double_ended();
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn with_double_ended(&mut self) -> &mut Self {
        self.double_ended = true;
        return self;
    }

    /// Make the arrow single-ended (the default).
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.arrow(Vec3::ZERO, Vec3::ONE, Color::GREEN)
    ///         .with_double_ended();
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn with_single_ended(&mut self) -> &mut Self {
        self.double_ended = false;
        return self;
    }
}

impl Drop for ArrowBuilder<'_, '_> {
    /// Draws the arrow, by drawing lines with the stored [`Gizmos`]
    fn drop(&mut self) {
        // first, draw the body of the arrow
        self.gizmos.line(self.start, self.end, self.color);
        // now the hard part is to draw the head in a sensible way
        // put us in a coordinate system where the arrow is pointing towards +x and ends at the origin
        let pointing = (self.end - self.start).normalize();
        let rotation = Quat::from_rotation_arc(Vec3::X, pointing);
        let tips = [
            Vec3::new(-1., 1., 0.),
            Vec3::new(-1., 0., 1.),
            Vec3::new(-1., -1., 0.),
            Vec3::new(-1., 0., -1.),
        ];
        // - extend the vectors so their length is `tip_length`
        let tips = tips.map(|v| (v.normalize() * self.tip_length));
        for v in tips {
            // - rotate the world so +x is facing in the same direction as the arrow
            // - translate over to the tip of the arrow
            // then actually draw the tips
            self.gizmos
                .line(self.end, self.end + (rotation * v), self.color);
        }
        if self.double_ended {
            // same thing but draw starting from the start and use the inverse rotation
            let rotation = Quat::from_rotation_arc(Vec3::NEG_X, pointing);
            for v in tips {
                self.gizmos
                    .line(self.start, self.start + (rotation * v), self.color);
            }
        }
    }
}

impl<'s> Gizmos<'s> {
    /// Draw an arrow in 3D, from `start` to `end`. Has four tips for convienent viewing from any direction.
    ///
    /// This should be called for each frame the arrow needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.arrow(Vec3::ZERO, Vec3::ONE, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn arrow(&mut self, start: Vec3, end: Vec3, color: Color) -> ArrowBuilder<'_, 's> {
        let length = (end - start).length();
        ArrowBuilder {
            gizmos: self,
            start,
            end,
            color,
            tip_length: length / 10.,
            double_ended: false,
        }
    }

    /// Draw an arrow in 2D (on the xy plane), from `start` to `end`.
    ///
    /// This should be called for each frame the arrow needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.arrow_2d(Vec2::ZERO, Vec2::X, Color::GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn arrow_2d(&mut self, start: Vec2, end: Vec2, color: Color) -> ArrowBuilder<'_, 's> {
        self.arrow(start.extend(0.), end.extend(0.), color)
    }
}
