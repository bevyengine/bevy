//! Additional [`Gizmos`] Functions -- Crosses
//!
//! Includes the implementation of [`Gizmos::cross`] and [`Gizmos::cross_2d`],
//! and assorted support items.

use crate::prelude::{GizmoConfigGroup, Gizmos};
use bevy_color::Color;
use bevy_math::{Mat2, Mat3, Quat, Vec2, Vec3};

impl<Config> Gizmos<'_, '_, Config>
where
    Config: GizmoConfigGroup,
{
    /// Draw a cross in 3D at `position`.
    ///
    /// This should be called for each frame the cross needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::WHITE;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.cross(Vec3::ZERO, Quat::IDENTITY, 0.5, WHITE);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn cross(
        &mut self,
        position: Vec3,
        rotation: Quat,
        half_size: f32,
        color: impl Into<Color>,
    ) {
        let axes = half_size * Mat3::from_quat(rotation);
        let local_x = axes.col(0);
        let local_y = axes.col(1);
        let local_z = axes.col(2);

        let color: Color = color.into();
        self.line(position + local_x, position - local_x, color);
        self.line(position + local_y, position - local_y, color);
        self.line(position + local_z, position - local_z, color);
    }

    /// Draw a cross in 2D (on the xy plane) at `position`.
    ///
    /// This should be called for each frame the cross needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::WHITE;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.cross_2d(Vec2::ZERO, 0.0, 0.5, WHITE);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn cross_2d(
        &mut self,
        position: Vec2,
        angle: f32,
        half_size: f32,
        color: impl Into<Color>,
    ) {
        let axes = half_size * Mat2::from_angle(angle);
        let local_x = axes.col(0);
        let local_y = axes.col(1);

        let color: Color = color.into();
        self.line_2d(position + local_x, position - local_x, color);
        self.line_2d(position + local_y, position - local_y, color);
    }
}
