//! Additional [`GizmoBuffer`] Functions -- Grids
//!
//! Includes the implementation of [`GizmoBuffer::grid`] and [`GizmoBuffer::grid_2d`].
//! and assorted support items.

use crate::{gizmos::GizmoBuffer, prelude::GizmoConfigGroup};
use bevy_color::Color;
use bevy_math::{ops, Isometry2d, Isometry3d, Quat, UVec2, UVec3, Vec2, Vec3};

/// A builder returned by [`GizmoBuffer::grid_3d`]
pub struct GridBuilder3d<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,
    isometry: Isometry3d,
    spacing: Vec3,
    cell_count: UVec3,
    skew: Vec3,
    outer_edges: [bool; 3],
    color: Color,
}
/// A builder returned by [`GizmoBuffer::grid`] and [`GizmoBuffer::grid_2d`]
pub struct GridBuilder2d<'a, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    gizmos: &'a mut GizmoBuffer<Config, Clear>,
    isometry: Isometry3d,
    spacing: Vec2,
    cell_count: UVec2,
    skew: Vec2,
    outer_edges: [bool; 2],
    color: Color,
}

impl<Config, Clear> GridBuilder3d<'_, Config, Clear>
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

    /// Declare that the outer edges of the grid parallel to the x axis should be drawn.
    /// By default, the outer edges will not be drawn.
    pub fn outer_edges_x(mut self) -> Self {
        self.outer_edges[0] = true;
        self
    }
    /// Declare that the outer edges of the grid parallel to the y axis should be drawn.
    /// By default, the outer edges will not be drawn.
    pub fn outer_edges_y(mut self) -> Self {
        self.outer_edges[1] = true;
        self
    }
    /// Declare that the outer edges of the grid parallel to the z axis should be drawn.
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

impl<Config, Clear> GridBuilder2d<'_, Config, Clear>
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

    /// Declare that the outer edges of the grid parallel to the x axis should be drawn.
    /// By default, the outer edges will not be drawn.
    pub fn outer_edges_x(mut self) -> Self {
        self.outer_edges[0] = true;
        self
    }
    /// Declare that the outer edges of the grid parallel to the y axis should be drawn.
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

impl<Config, Clear> Drop for GridBuilder3d<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draws a grid, by drawing lines with the stored [`GizmoBuffer`]
    fn drop(&mut self) {
        draw_grid(
            self.gizmos,
            self.isometry,
            self.spacing,
            self.cell_count,
            self.skew,
            self.outer_edges,
            self.color,
        );
    }
}

impl<Config, Clear> Drop for GridBuilder2d<'_, Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    fn drop(&mut self) {
        draw_grid(
            self.gizmos,
            self.isometry,
            self.spacing.extend(0.),
            self.cell_count.extend(0),
            self.skew.extend(0.),
            [self.outer_edges[0], self.outer_edges[1], true],
            self.color,
        );
    }
}

impl<Config, Clear> GizmoBuffer<Config, Clear>
where
    Config: GizmoConfigGroup,
    Clear: 'static + Send + Sync,
{
    /// Draw a 2D grid in 3D.
    ///
    /// This should be called for each frame the grid needs to be rendered.
    ///
    /// The grid's default orientation aligns with the XY-plane.
    ///
    /// # Arguments
    ///
    /// - `isometry` defines the translation and rotation of the grid.
    ///   - the translation specifies the center of the grid
    ///   - defines the orientation of the grid, by default we assume the grid is contained in a
    ///     plane parallel to the XY plane
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
    ///         Isometry3d::IDENTITY,
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
        isometry: impl Into<Isometry3d>,
        cell_count: UVec2,
        spacing: Vec2,
        color: impl Into<Color>,
    ) -> GridBuilder2d<'_, Config, Clear> {
        GridBuilder2d {
            gizmos: self,
            isometry: isometry.into(),
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
    /// - `isometry` defines the translation and rotation of the grid.
    ///   - the translation specifies the center of the grid
    ///   - defines the orientation of the grid, by default we assume the grid is aligned with all axes
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
    ///         Isometry3d::IDENTITY,
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
        isometry: impl Into<Isometry3d>,
        cell_count: UVec3,
        spacing: Vec3,
        color: impl Into<Color>,
    ) -> GridBuilder3d<'_, Config, Clear> {
        GridBuilder3d {
            gizmos: self,
            isometry: isometry.into(),
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
    /// - `isometry` defines the translation and rotation of the grid.
    ///   - the translation specifies the center of the grid
    ///   - defines the orientation of the grid, by default we assume the grid is aligned with all axes
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
    ///         Isometry2d::IDENTITY,
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
        isometry: impl Into<Isometry2d>,
        cell_count: UVec2,
        spacing: Vec2,
        color: impl Into<Color>,
    ) -> GridBuilder2d<'_, Config, Clear> {
        let isometry = isometry.into();
        GridBuilder2d {
            gizmos: self,
            isometry: Isometry3d::new(
                isometry.translation.extend(0.0),
                Quat::from_rotation_z(isometry.rotation.as_radians()),
            ),
            spacing,
            cell_count,
            skew: Vec2::ZERO,
            outer_edges: [false, false],
            color: color.into(),
        }
    }
}

fn draw_grid<Config, Clear>(
    gizmos: &mut GizmoBuffer<Config, Clear>,
    isometry: Isometry3d,
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
    let skew_tan = Vec3::from(skew.to_array().map(ops::tan));
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

    #[inline]
    fn cell_count_to_line_count(include_outer: bool, cell_count: u32) -> u32 {
        if include_outer {
            cell_count.saturating_add(1)
        } else {
            cell_count.saturating_sub(1).max(1)
        }
    }

    let x_line_count = UVec2::new(
        cell_count_to_line_count(outer_edges[0], cell_count.y),
        cell_count_to_line_count(outer_edges[0], cell_count.z),
    );
    let y_line_count = UVec2::new(
        cell_count_to_line_count(outer_edges[1], cell_count.z),
        cell_count_to_line_count(outer_edges[1], cell_count.x),
    );
    let z_line_count = UVec2::new(
        cell_count_to_line_count(outer_edges[2], cell_count.x),
        cell_count_to_line_count(outer_edges[2], cell_count.y),
    );

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
    let x_lines = iter_lines(dx, dy, dz, x_line_count, cell_count.x, x_start);
    // Lines along the y direction
    let y_lines = iter_lines(dy, dz, dx, y_line_count, cell_count.y, y_start);
    // Lines along the z direction
    let z_lines = iter_lines(dz, dx, dy, z_line_count, cell_count.z, z_start);

    x_lines
        .chain(y_lines)
        .chain(z_lines)
        .map(|vec3s| vec3s.map(|vec3| isometry * vec3))
        .for_each(|[start, end]| {
            gizmos.line(start, end, color);
        });
}
