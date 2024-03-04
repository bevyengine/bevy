//! Additional [`Gizmos`] Functions -- Arrows
//!
//! Includes the implementation of [`Gizmos::arrow`] and [`Gizmos::arrow_2d`],
//! and assorted support items.

use crate::prelude::{GizmoConfigGroup, Gizmos};
use bevy_color::{
    palettes::basic::{BLUE, GREEN, RED},
    Color,
};
use bevy_math::{Quat, Vec2, Vec3};
use bevy_transform::TransformPoint;

/// A builder returned by [`Gizmos::arrow`] and [`Gizmos::arrow_2d`]
pub struct ArrowBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,
    start: Vec3,
    end: Vec3,
    color: Color,
    tip_length: f32,
}

impl<T: GizmoConfigGroup> ArrowBuilder<'_, '_, '_, T> {
    /// Change the length of the tips to be `length`.
    /// The default tip length is [length of the arrow]/10.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.arrow(Vec3::ZERO, Vec3::ONE, GREEN)
    ///         .with_tip_length(3.);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    #[doc(alias = "arrow_head_length")]
    pub fn with_tip_length(mut self, length: f32) -> Self {
        self.tip_length = length;
        self
    }
}

impl<T: GizmoConfigGroup> Drop for ArrowBuilder<'_, '_, '_, T> {
    /// Draws the arrow, by drawing lines with the stored [`Gizmos`]
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }
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
        // - rotate the world so +x is facing in the same direction as the arrow
        // - translate over to the tip of the arrow
        let tips = tips.map(|v| rotation * (v.normalize() * self.tip_length) + self.end);
        for v in tips {
            // then actually draw the tips
            self.gizmos.line(self.end, v, self.color);
        }
    }
}

impl<'w, 's, T: GizmoConfigGroup> Gizmos<'w, 's, T> {
    /// Draw an arrow in 3D, from `start` to `end`. Has four tips for convenient viewing from any direction.
    ///
    /// This should be called for each frame the arrow needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.arrow(Vec3::ZERO, Vec3::ONE, GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn arrow(
        &mut self,
        start: Vec3,
        end: Vec3,
        color: impl Into<Color>,
    ) -> ArrowBuilder<'_, 'w, 's, T> {
        let length = (end - start).length();
        ArrowBuilder {
            gizmos: self,
            start,
            end,
            color: color.into(),
            tip_length: length / 10.,
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
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.arrow_2d(Vec2::ZERO, Vec2::X, GREEN);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn arrow_2d(
        &mut self,
        start: Vec2,
        end: Vec2,
        color: impl Into<Color>,
    ) -> ArrowBuilder<'_, 'w, 's, T> {
        self.arrow(start.extend(0.), end.extend(0.), color)
    }
}

impl<'w, 's, T: GizmoConfigGroup> Gizmos<'w, 's, T> {
    /// Draw a set of axes local to the given transform (`transform`), with length scaled by a factor
    /// of `base_length`.
    ///
    /// This should be called for each frame the axes need to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_ecs::prelude::*;
    /// # use bevy_transform::components::Transform;
    /// # #[derive(Component)]
    /// # struct MyComponent;
    /// fn draw_axes(
    ///     mut gizmos: Gizmos,
    ///     query: Query<&Transform, With<MyComponent>>,
    /// ) {
    ///     for &transform in &query {
    ///         gizmos.axes(transform, 1.);
    ///     }
    /// }
    /// # bevy_ecs::system::assert_is_system(draw_axes);
    /// ```
    pub fn axes(&mut self, transform: impl TransformPoint, base_length: f32) {
        let start = transform.transform_point(Vec3::ZERO);
        let end_x = transform.transform_point(base_length * Vec3::X);
        let end_y = transform.transform_point(base_length * Vec3::Y);
        let end_z = transform.transform_point(base_length * Vec3::Z);

        self.arrow(start, end_x, RED);
        self.arrow(start, end_y, GREEN);
        self.arrow(start, end_z, BLUE);
    }

    pub fn axes_2d(&mut self, transform: impl TransformPoint, base_length: f32) {
        let start = transform.transform_point(Vec2::ZERO);
        let end_x = transform.transform_point(base_length * Vec2::X);
        let end_y = transform.transform_point(base_length * Vec2::Y);

        self.arrow_2d(start, end_x, RED);
        self.arrow_2d(start, end_y, GREEN);
    }
}
