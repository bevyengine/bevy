//! Additional Gizmo Functions -- Arrows

use crate::prelude::Gizmos;
use bevy_math::{Quat, Vec2, Vec3};
use bevy_render::color::Color;

pub struct ArrowBuilder<'a, 's> {
    gizmos: &'a mut Gizmos<'s>,
    start: Vec3,
    end: Vec3,
    color: Color,
    tip_length: f32,
}

impl Drop for ArrowBuilder<'_, '_> {
    fn drop(&mut self) {
        // draw the body of the arrow
        self.gizmos.line(self.start, self.end, self.color);
        // now the hard part is to draw the head in a sensible way
        // put us in a coordinate system where the arrow is pointing up and ends at the origin
        let pointing = (self.end - self.start).normalize();
        let rotation = Quat::from_rotation_arc(Vec3::X, pointing);
        let tips = [(1, 0), (0, 1), (-1, 0), (0, -1)]
            .into_iter()
            .map(|(y, z)| Vec3 {
                x: -1.,
                y: y as f32,
                z: z as f32,
            })
            .map(|v| v * self.tip_length)
            .map(|v| rotation.mul_vec3(v))
            .map(|v| v + self.end);
        for v in tips {
            self.gizmos.line(self.end, v, self.color);
        }
    }
}

impl<'s> Gizmos<'s> {
    /// Draw an arrow in 3D, from `start` to `end`. Has four tips for convienent viewing from any direction.
    ///
    /// This should be called for each frame the arrow needs to be rendered.
    ///
    /// # example
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
        // self.line_2d(start, end, color);
        let length = (end - start).length();
        ArrowBuilder {
            gizmos: self,
            start,
            end,
            color,
            tip_length: length / 10.,
        }
    }

    /// Draw an arrow in 2D (on the xy plane), from `start` to `end`.
    ///
    /// This should be called for each frame the arrow needs to be rendered.
    ///
    /// # example
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
