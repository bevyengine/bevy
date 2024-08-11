//! Additional [`Gizmos`] Functions -- Crosses
//!
//! Includes the implementation of [`Gizmos::cross`] and [`Gizmos::cross_2d`],
//! and assorted support items.

use crate::prelude::{GizmoConfigGroup, Gizmos};
use bevy_color::Color;
use bevy_math::{Isometry2d, Isometry3d, Vec2, Vec3};

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
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::WHITE;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.cross(Vec3::ZERO, Quat::IDENTITY, 0.5, WHITE);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn cross(&mut self, isometry: Isometry3d, half_size: f32, color: impl Into<Color>) {
        let color: Color = color.into();
        [Vec3::X, Vec3::Y, Vec3::Z]
            .map(|axis| axis * half_size)
            .into_iter()
            .for_each(|axis| {
                self.line(isometry * axis, isometry * (-axis), color);
            });
    }

    /// Draw a cross in 2D (on the xy plane) at `position`.
    ///
    /// This should be called for each frame the cross needs to be rendered.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::WHITE;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.cross_2d(Vec2::ZERO, 0.0, 0.5, WHITE);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn cross_2d(&mut self, isometry: Isometry2d, half_size: f32, color: impl Into<Color>) {
        let color: Color = color.into();
        [Vec2::X, Vec2::Y]
            .map(|axis| axis * half_size)
            .into_iter()
            .for_each(|axis| {
                self.line_2d(isometry * axis, isometry * (-axis), color);
            });
    }
}
