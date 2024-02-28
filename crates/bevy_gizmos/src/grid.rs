//! Additional [`Gizmos`] Functions -- Grids
//!
//! Includes the implementation of[`Gizmos::grid`] and [`Gizmos::grid_2d`].
//! and assorted support items.

use crate::prelude::{GizmoConfigGroup, Gizmos};
use bevy_color::LinearRgba;
use bevy_math::{Quat, UVec2, Vec2, Vec3};

/// A builder returned by [`Gizmos::grid`] and [`Gizmos::grid_2d`]
pub struct GridBuilder<'a, 'w, 's, T: GizmoConfigGroup> {
    gizmos: &'a mut Gizmos<'w, 's, T>,
    position: Vec3,
    rotation: Quat,
    spacing: Vec2,
    cell_count: UVec2,
    skew: Vec2,
    outer_edges: bool,
    color: LinearRgba,
}

impl<T: GizmoConfigGroup> GridBuilder<'_, '_, '_, T> {
    /// Skews the grid by `tan(skew)` in the x direction.
    /// `skew` is in radians
    pub fn skew_x(mut self, skew: f32) -> Self {
        self.skew.x = skew;
        self
    }
    /// Skews the grid by `tan(skew)` in the x direction.
    /// `skew` is in radians
    pub fn skew_y(mut self, skew: f32) -> Self {
        self.skew.y = skew;
        self
    }
    /// Skews the grid by `tan(skew)` in the x and y directions.
    /// `skew` is in radians
    pub fn skew(mut self, skew: Vec2) -> Self {
        self.skew = skew;
        self
    }

    /// Toggle whether the outer edges of the grid should be drawn.
    /// By default, the outer edges will not be drawn.
    pub fn outer_edges(mut self, outer_edges: bool) -> Self {
        self.outer_edges = outer_edges;
        self
    }
}

impl<T: GizmoConfigGroup> Drop for GridBuilder<'_, '_, '_, T> {
    /// Draws a grid, by drawing lines with the stored [`Gizmos`]
    fn drop(&mut self) {
        if !self.gizmos.enabled {
            return;
        }

        // Offset between two adjacent grid cells along the x/y-axis and accounting for skew.
        let dx = Vec3::new(self.spacing.x, self.spacing.x * self.skew.y.tan(), 0.);
        let dy = Vec3::new(self.spacing.y * self.skew.x.tan(), self.spacing.y, 0.);

        // Bottom-left corner of the grid
        let grid_start = self.position
            - self.cell_count.x as f32 / 2.0 * dx
            - self.cell_count.y as f32 / 2.0 * dy;

        let (line_count, vertical_start, horizontal_start) = if self.outer_edges {
            (self.cell_count + UVec2::ONE, grid_start, grid_start)
        } else {
            (
                self.cell_count.saturating_sub(UVec2::ONE),
                grid_start + dx,
                grid_start + dy,
            )
        };

        // Vertical lines
        let dline = dy * self.cell_count.y as f32;
        for i in 0..line_count.x {
            let i = i as f32;
            let line_start = vertical_start + i * dx;
            let line_end = line_start + dline;

            self.gizmos.line(
                self.rotation * line_start,
                self.rotation * line_end,
                self.color,
            );
        }

        // Horizontal lines
        let dline = dx * self.cell_count.x as f32;
        for i in 0..line_count.y {
            let i = i as f32;
            let line_start = horizontal_start + i * dy;
            let line_end = line_start + dline;

            self.gizmos.line(
                self.rotation * line_start,
                self.rotation * line_end,
                self.color,
            );
        }
    }
}

impl<'w, 's, T: GizmoConfigGroup> Gizmos<'w, 's, T> {
    /// Draw a 2D grid in 3D.
    ///
    /// This should be called for each frame the grid needs to be rendered.
    ///
    /// # Arguments
    ///
    /// - `position`: The center point of the grid.
    /// - `rotation`: defines the orientation of the grid, by default we assume the grid is contained in a plane parallel to the XY plane.
    /// - `cell_count`: defines the amount of cells in the x and y axes
    /// - `spacing`: defines the distance between cells along the x and y axes
    /// - `color`: color of the grid
    ///
    /// # Builder methods
    ///
    /// - The skew of the grid can be adjusted using the `.skew(...)`, `.skew_x(...)` or `.skew_y(...)` methods. They behave very similar to their CSS equivalents.
    /// - The outer edges can be toggled on or off using `.outer_edges(...)`.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.grid(
    ///         Vec3::ZERO,
    ///         Quat::IDENTITY,
    ///         UVec2::new(10, 10),
    ///         Vec2::splat(2.),
    ///         LegacyColor::GREEN
    ///         )
    ///         .skew_x(0.25)
    ///         .outer_edges(true);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn grid(
        &mut self,
        position: Vec3,
        rotation: Quat,
        cell_count: UVec2,
        spacing: Vec2,
        color: impl Into<LinearRgba>,
    ) -> GridBuilder<'_, 'w, 's, T> {
        GridBuilder {
            gizmos: self,
            position,
            rotation,
            spacing,
            cell_count,
            skew: Vec2::ZERO,
            outer_edges: false,
            color: color.into(),
        }
    }

    /// Draw a grid in 2D.
    ///
    /// This should be called for each frame the grid needs to be rendered.
    ///
    /// # Arguments
    ///
    /// - `position`: The center point of the grid.
    /// - `rotation`: defines the orientation of the grid.
    /// - `cell_count`: defines the amount of cells in the x and y axes
    /// - `spacing`: defines the distance between cells along the x and y axes
    /// - `color`: color of the grid
    ///
    /// # Builder methods
    ///
    /// - The skew of the grid can be adjusted using the `.skew(...)`, `.skew_x(...)` or `.skew_y(...)` methods. They behave very similar to their CSS equivalents.
    /// - The outer edges can be toggled on or off using `.outer_edges(...)`.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.grid_2d(
    ///         Vec2::ZERO,
    ///         0.0,
    ///         UVec2::new(10, 10),
    ///         Vec2::splat(1.),
    ///         LegacyColor::GREEN
    ///         )
    ///         .skew_x(0.25)
    ///         .outer_edges(true);
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn grid_2d(
        &mut self,
        position: Vec2,
        rotation: f32,
        cell_count: UVec2,
        spacing: Vec2,
        color: impl Into<LinearRgba>,
    ) -> GridBuilder<'_, 'w, 's, T> {
        GridBuilder {
            gizmos: self,
            position: position.extend(0.),
            rotation: Quat::from_rotation_z(rotation),
            spacing,
            cell_count,
            skew: Vec2::ZERO,
            outer_edges: false,
            color: color.into(),
        }
    }
}
