//! Additional [`Gizmos`] Functions -- Grids
//!
//! Includes the implementation of [`Gizmos::grid`] and [`Gizmos::grid_2d`].
//! and assorted support items.

use crate::prelude::{GizmoConfigGroup, Gizmos};
use bevy_color::Color;
use bevy_math::{Quat, UVec2, UVec3, Vec2, Vec3, Vec3Swizzles};

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
    color: Color,
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
    color: Color,
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
        color: impl Into<Color>,
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
        color: impl Into<Color>,
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
        color: impl Into<Color>,
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
    color: Color,
) where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    if !gizmos.enabled {
        return;
    }

    #[inline]
    fn or_zero(cond: bool, val: Vec3) -> Vec3 {
        if cond {
            val
        } else {
            Vec3::ZERO
        }
    }

    // Offset between two adjacent grid cells along the x/y-axis and accounting for skew.
    let skew_tan = Vec3::from(skew.to_array().map(f32::tan));
    let dx = or_zero(
        cell_count.x != 0,
        spacing.x * Vec3::new(1., skew_tan.y, skew_tan.z),
    );
    let dy = or_zero(
        cell_count.y != 0,
        spacing.y * Vec3::new(skew_tan.x, 1., skew_tan.z),
    );
    let dz = or_zero(
        cell_count.z != 0,
        spacing.z * Vec3::new(skew_tan.x, skew_tan.y, 1.),
    );

    // Bottom-left-front corner of the grid
    let cell_count_half = cell_count.as_vec3() * 0.5;
    let grid_start = -cell_count_half.x * dx - cell_count_half.y * dy - cell_count_half.z * dz;

    let outer_edges_u32 = UVec3::from(outer_edges.map(|v| v as u32));
    let line_count = outer_edges_u32 * cell_count.saturating_add(UVec3::ONE)
        + (UVec3::ONE - outer_edges_u32) * cell_count.saturating_sub(UVec3::ONE);

    let x_start = grid_start + or_zero(!outer_edges[0], dy + dz);
    let y_start = grid_start + or_zero(!outer_edges[1], dx + dz);
    let z_start = grid_start + or_zero(!outer_edges[2], dx + dy);

    fn iter_lines(
        delta_a: Vec3,
        delta_b: Vec3,
        delta_c: Vec3,
        line_count: UVec2,
        cell_count: u32,
        start: Vec3,
    ) -> impl Iterator<Item = [Vec3; 2]> {
        let dline = delta_a * cell_count as f32;
        (0..line_count.x).map(|v| v as f32).flat_map(move |b| {
            (0..line_count.y).map(|v| v as f32).map(move |c| {
                let line_start = start + b * delta_b + c * delta_c;
                let line_end = line_start + dline;
                [line_start, line_end]
            })
        })
    }

    // Lines along the x direction
    let x_lines = iter_lines(dx, dy, dz, line_count.yz(), cell_count.x, x_start);
    // Lines along the y direction
    let y_lines = iter_lines(dy, dz, dx, line_count.zx(), cell_count.y, y_start);
    // Lines along the z direction
    let z_lines = iter_lines(dz, dx, dy, line_count.xy(), cell_count.z, z_start);
    x_lines
        .chain(y_lines)
        .chain(z_lines)
        .map(|ps| ps.map(|p| position + rotation * p))
        .for_each(|[start, end]| {
            gizmos.line(start, end, color);
        });
}
