//! Additional [`Gizmos`] Functions -- Grids
//!
//! Includes the implementation of[`Gizmos::grid`] and [`Gizmos::grid_2d`].
//! and assorted support items.

use crate::prelude::{GizmoConfigGroup, Gizmos};
use bevy_color::LinearRgba;
use bevy_math::{Quat, UVec2, UVec3, Vec2, Vec3};

/// A builder returned by [`Gizmos::grid_3d`]
pub struct GridBuilder3d<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,
    position: Vec3,
    rotation: Quat,
    spacing: Vec3,
    cell_count: UVec3,
    skew: Vec3,
    outer_edges: [bool; 3],
    color: LinearRgba,
}
/// A builder returned by [`Gizmos::grid`] and [`Gizmos::grid_2d`]
pub struct GridBuilder2d<'a, 'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut Gizmos<'w, 's, Config, Clear>,
    position: Vec3,
    rotation: Quat,
    spacing: Vec2,
    cell_count: UVec2,
    skew: Vec2,
    outer_edges: [bool; 2],
    color: LinearRgba,
}

impl<Config, Clear> GridBuilder3d<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Skews the grid by `tan(skew)` in the x direction.
    /// `skew` is in radians
    pub fn skew_x(mut self, skew: f32) -> Self {
        self.skew.x = skew;
        self
    }
    /// Skews the grid by `tan(skew)` in the y direction.
    /// `skew` is in radians
    pub fn skew_y(mut self, skew: f32) -> Self {
        self.skew.y = skew;
        self
    }
    /// Skews the grid by `tan(skew)` in the z direction.
    /// `skew` is in radians
    pub fn skew_z(mut self, skew: f32) -> Self {
        self.skew.z = skew;
        self
    }
    /// Skews the grid by `tan(skew)` in the x, y and z directions.
    /// `skew` is in radians
    pub fn skew(mut self, skew: Vec3) -> Self {
        self.skew = skew;
        self
    }

    /// Declare that the outer edges of the grid along the x axis should be drawn.
    /// By default, the outer edges will not be drawn.
    pub fn outer_edges_x(mut self) -> Self {
        self.outer_edges[0] = true;
        self
    }
    /// Declare that the outer edges of the grid along the y axis should be drawn.
    /// By default, the outer edges will not be drawn.
    pub fn outer_edges_y(mut self) -> Self {
        self.outer_edges[1] = true;
        self
    }
    /// Declare that the outer edges of the grid along the z axis should be drawn.
    /// By default, the outer edges will not be drawn.
    pub fn outer_edges_z(mut self) -> Self {
        self.outer_edges[2] = true;
        self
    }
    /// Declare that all outer edges of the grid should be drawn.
    /// By default, the outer edges will not be drawn.
    pub fn outer_edges(mut self) -> Self {
        self.outer_edges.fill(true);
        self
    }
}

impl<Config, Clear> GridBuilder2d<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Skews the grid by `tan(skew)` in the x direction.
    /// `skew` is in radians
    pub fn skew_x(mut self, skew: f32) -> Self {
        self.skew.x = skew;
        self
    }
    /// Skews the grid by `tan(skew)` in the y direction.
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

    /// Declare that the outer edges of the grid along the x axis should be drawn.
    /// By default, the outer edges will not be drawn.
    pub fn outer_edges_x(mut self) -> Self {
        self.outer_edges[0] = true;
        self
    }
    /// Declare that the outer edges of the grid along the y axis should be drawn.
    /// By default, the outer edges will not be drawn.
    pub fn outer_edges_y(mut self) -> Self {
        self.outer_edges[1] = true;
        self
    }
    /// Declare that all outer edges of the grid should be drawn.
    /// By default, the outer edges will not be drawn.
    pub fn outer_edges(mut self) -> Self {
        self.outer_edges.fill(true);
        self
    }
}

impl<Config, Clear> Drop for GridBuilder3d<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draws a grid, by drawing lines with the stored [`Gizmos`]
    fn drop(&mut self) {
        draw_grid(
            self.gizmos,
            self.position,
            self.rotation,
            self.spacing,
            self.cell_count,
            self.skew,
            self.outer_edges,
            self.color,
        );
    }
}

impl<Config, Clear> Drop for GridBuilder2d<'_, '_, '_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        draw_grid(
            self.gizmos,
            self.position,
            self.rotation,
            self.spacing.extend(0.),
            self.cell_count.extend(0),
            self.skew.extend(0.),
            [self.outer_edges[0], self.outer_edges[1], true],
            self.color,
        );
    }
}
impl<'w, 's, Config, Clear> Gizmos<'w, 's, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
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
    /// - All outer edges can be toggled on or off using `.outer_edges(...)`. Alternatively you can use `.outer_edges_x(...)` or `.outer_edges_y(...)` to toggle the outer edges along an axis.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.grid(
    ///         Vec3::ZERO,
    ///         Quat::IDENTITY,
    ///         UVec2::new(10, 10),
    ///         Vec2::splat(2.),
    ///         GREEN
    ///         )
    ///         .skew_x(0.25)
    ///         .outer_edges();
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
    ) -> GridBuilder2d<'_, 'w, 's, Config, Clear> {
        GridBuilder2d {
            gizmos: self,
            position,
            rotation,
            spacing,
            cell_count,
            skew: Vec2::ZERO,
            outer_edges: [false, false],
            color: color.into(),
        }
    }

    /// Draw a 3D grid of voxel-like cells.
    ///
    /// This should be called for each frame the grid needs to be rendered.
    ///
    /// # Arguments
    ///
    /// - `position`: The center point of the grid.
    /// - `rotation`: defines the orientation of the grid, by default we assume the grid is contained in a plane parallel to the XY plane.
    /// - `cell_count`: defines the amount of cells in the x, y and z axes
    /// - `spacing`: defines the distance between cells along the x, y and z axes
    /// - `color`: color of the grid
    ///
    /// # Builder methods
    ///
    /// - The skew of the grid can be adjusted using the `.skew(...)`, `.skew_x(...)`, `.skew_y(...)` or  `.skew_z(...)` methods. They behave very similar to their CSS equivalents.
    /// - All outer edges can be toggled on or off using `.outer_edges(...)`. Alternatively you can use `.outer_edges_x(...)`, `.outer_edges_y(...)` or `.outer_edges_z(...)` to toggle the outer edges along an axis.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.grid_3d(
    ///         Vec3::ZERO,
    ///         Quat::IDENTITY,
    ///         UVec3::new(10, 2, 10),
    ///         Vec3::splat(2.),
    ///         GREEN
    ///         )
    ///         .skew_x(0.25)
    ///         .outer_edges();
    /// }
    /// # bevy_ecs::system::assert_is_system(system);
    /// ```
    pub fn grid_3d(
        &mut self,
        position: Vec3,
        rotation: Quat,
        cell_count: UVec3,
        spacing: Vec3,
        color: impl Into<LinearRgba>,
    ) -> GridBuilder3d<'_, 'w, 's, Config, Clear> {
        GridBuilder3d {
            gizmos: self,
            position,
            rotation,
            spacing,
            cell_count,
            skew: Vec3::ZERO,
            outer_edges: [false, false, false],
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
    /// - All outer edges can be toggled on or off using `.outer_edges(...)`. Alternatively you can use `.outer_edges_x(...)` or `.outer_edges_y(...)` to toggle the outer edges along an axis.
    ///
    /// # Example
    /// ```
    /// # use bevy_gizmos::prelude::*;
    /// # use bevy_render::prelude::*;
    /// # use bevy_math::prelude::*;
    /// # use bevy_color::palettes::basic::GREEN;
    /// fn system(mut gizmos: Gizmos) {
    ///     gizmos.grid_2d(
    ///         Vec2::ZERO,
    ///         0.0,
    ///         UVec2::new(10, 10),
    ///         Vec2::splat(1.),
    ///         GREEN
    ///         )
    ///         .skew_x(0.25)
    ///         .outer_edges();
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
    ) -> GridBuilder2d<'_, 'w, 's, Config, Clear> {
        GridBuilder2d {
            gizmos: self,
            position: position.extend(0.),
            rotation: Quat::from_rotation_z(rotation),
            spacing,
            cell_count,
            skew: Vec2::ZERO,
            outer_edges: [false, false],
            color: color.into(),
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_grid<Config, Clear>(
    gizmos: &mut Gizmos<'_, '_, Config, Clear>,
    position: Vec3,
    rotation: Quat,
    spacing: Vec3,
    cell_count: UVec3,
    skew: Vec3,
    outer_edges: [bool; 3],
    color: LinearRgba,
) where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    if !gizmos.enabled {
        return;
    }

    // Offset between two adjacent grid cells along the x/y-axis and accounting for skew.
    let dx = spacing.x
        * Vec3::new(1., skew.y.tan(), skew.z.tan())
        * if cell_count.x != 0 { 1. } else { 0. };
    let dy = spacing.y
        * Vec3::new(skew.x.tan(), 1., skew.z.tan())
        * if cell_count.y != 0 { 1. } else { 0. };
    let dz = spacing.z
        * Vec3::new(skew.x.tan(), skew.y.tan(), 1.)
        * if cell_count.z != 0 { 1. } else { 0. };

    // Bottom-left-front corner of the grid
    let grid_start = position
        - cell_count.x as f32 / 2.0 * dx
        - cell_count.y as f32 / 2.0 * dy
        - cell_count.z as f32 / 2.0 * dz;

    let line_count = UVec3::new(
        if outer_edges[0] {
            cell_count.x + 1
        } else {
            cell_count.x.saturating_sub(1)
        },
        if outer_edges[1] {
            cell_count.y + 1
        } else {
            cell_count.y.saturating_sub(1)
        },
        if outer_edges[2] {
            cell_count.z + 1
        } else {
            cell_count.z.saturating_sub(1)
        },
    );
    let x_start = grid_start + if outer_edges[0] { Vec3::ZERO } else { dy + dz };
    let y_start = grid_start + if outer_edges[1] { Vec3::ZERO } else { dx + dz };
    let z_start = grid_start + if outer_edges[2] { Vec3::ZERO } else { dx + dy };

    // Lines along the x direction
    let dline = dx * cell_count.x as f32;
    for iy in 0..line_count.y {
        let iy = iy as f32;
        for iz in 0..line_count.z {
            let iz = iz as f32;
            let line_start = x_start + iy * dy + iz * dz;
            let line_end = line_start + dline;

            gizmos.line(rotation * line_start, rotation * line_end, color);
        }
    }
    // Lines along the y direction
    let dline = dy * cell_count.y as f32;
    for ix in 0..line_count.x {
        let ix = ix as f32;
        for iz in 0..line_count.z {
            let iz = iz as f32;
            let line_start = y_start + ix * dx + iz * dz;
            let line_end = line_start + dline;

            gizmos.line(rotation * line_start, rotation * line_end, color);
        }
    }
    // Lines along the z direction
    let dline = dz * cell_count.z as f32;
    for ix in 0..line_count.x {
        let ix = ix as f32;
        for iy in 0..line_count.y {
            let iy = iy as f32;
            let line_start = z_start + ix * dx + iy * dy;
            let line_end = line_start + dline;

            gizmos.line(rotation * line_start, rotation * line_end, color);
        }
    }
}
