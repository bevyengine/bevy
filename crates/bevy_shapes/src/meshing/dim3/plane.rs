use crate::primitives::Plane3d;
use bevy_math::{Dir3, Vec2};
use bevy_mesh::{Mesh, MeshBuilder, Meshable};
use bevy_reflect::prelude::*;

/// A builder used for creating a [`Mesh`] with a [`Plane3d`] shape.
#[derive(Clone, Copy, Debug, Default, Reflect)]
#[reflect(Default, Debug, Clone)]
pub struct PlaneMeshBuilder {
    /// The [`Plane3d`] shape.
    pub plane: Plane3d,
    /// The number of subdivisions along the X axis.
    ///
    /// 0 - is the original plane geometry, the 4 points in the XZ plane.
    ///
    /// 1 - adds a vertex in the middle of the X axis, resulting in a plane with 2 quads / 4 triangles, and a new edge along the Z axis.
    ///
    /// 2 - adds 2 vertices along the X axis, resulting in a plane with 3 quads / 6 triangles.
    ///
    /// and so on...
    pub subdivisions_x: u32,

    /// The number of subdivisions along the Z axis.
    ///
    /// 0 - is the original plane geometry, the 4 points in the XZ plane.
    ///
    /// 1 - adds a vertex in the middle of the Z axis, resulting in a plane with 2 quads / 4 triangles, and a new edge along the X axis.
    ///
    /// 2 - adds 2 vertices along the Z axis, resulting in a plane with 3 quads / 6 triangles.
    ///
    /// and so on...
    pub subdivisions_z: u32,
}

impl PlaneMeshBuilder {
    /// Creates a new [`PlaneMeshBuilder`] from a given normal and size.
    #[inline]
    pub fn new(normal: Dir3, size: Vec2) -> Self {
        Self {
            plane: Plane3d {
                normal,
                half_size: size / 2.0,
            },
            subdivisions_x: 0,
            subdivisions_z: 0,
        }
    }

    /// Creates a new [`PlaneMeshBuilder`] from the given size, with the normal pointing upwards.
    #[inline]
    pub fn from_size(size: Vec2) -> Self {
        Self {
            plane: Plane3d {
                half_size: size / 2.0,
                ..Default::default()
            },
            subdivisions_x: 0,
            subdivisions_z: 0,
        }
    }

    /// Creates a new [`PlaneMeshBuilder`] from the given length, with the normal pointing upwards,
    /// and the resulting [`PlaneMeshBuilder`] being a square.
    #[inline]
    pub fn from_length(length: f32) -> Self {
        Self {
            plane: Plane3d {
                half_size: Vec2::splat(length) / 2.0,
                ..Default::default()
            },
            subdivisions_x: 0,
            subdivisions_z: 0,
        }
    }

    /// Sets the normal of the plane, aka the direction the plane is facing.
    #[inline]
    #[doc(alias = "facing")]
    pub fn normal(mut self, normal: Dir3) -> Self {
        self.plane = Plane3d {
            normal,
            ..self.plane
        };
        self
    }

    /// Sets the size of the plane mesh.
    #[inline]
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.plane.half_size = Vec2::new(width, height) / 2.0;
        self
    }

    /// Sets the subdivisions of the plane mesh.
    ///
    /// 0 - is the original plane geometry, the 4 points in the XZ plane.
    ///
    /// 1 - is split by 1 line in the middle of the plane on both the X axis and the Z axis,
    ///     resulting in a plane with 4 quads / 8 triangles.
    ///
    /// 2 - is a plane split by 2 lines on both the X and Z axes, subdividing the plane into 3
    ///     equal sections along each axis, resulting in a plane with 9 quads / 18 triangles.
    #[inline]
    pub fn subdivisions(mut self, subdivisions: u32) -> Self {
        self.subdivisions_x = subdivisions;
        self.subdivisions_z = subdivisions;
        self
    }

    #[inline]
    /// The number of subdivisions along the X axis.
    ///
    /// 0 - is the original plane geometry, the 4 points in the XZ plane.
    ///
    /// 1 - adds a vertex in the middle of the X axis, resulting in a plane with 2 quads / 4 triangles, and a new edge along the Z axis.
    ///
    /// 2 - adds 2 vertices along the X axis, resulting in a plane with 3 quads / 6 triangles.
    ///
    /// and so on...
    pub fn subdivisions_x(mut self, subdivisions: u32) -> Self {
        self.subdivisions_x = subdivisions;
        self
    }

    #[inline]
    /// The number of subdivisions along the Z axis.
    ///
    /// 0 - is the original plane geometry, the 4 points in the XZ plane.
    ///
    /// 1 - adds a vertex in the middle of the Z axis, resulting in a plane with 2 quads / 4 triangles, and a new edge along the X axis.
    ///
    /// 2 - adds 2 vertices along the Z axis, resulting in a plane with 3 quads / 6 triangles.
    ///
    /// and so on...
    pub fn subdivisions_z(mut self, subdivisions: u32) -> Self {
        self.subdivisions_z = subdivisions;
        self
    }
}

impl MeshBuilder for PlaneMeshBuilder {
    fn build(&self) -> Mesh {
        Mesh::plane_mesh(
            self.plane.normal,
            self.plane.half_size,
            self.subdivisions_x,
            self.subdivisions_z,
        )
    }
}

impl Meshable for Plane3d {
    type Output = PlaneMeshBuilder;

    fn mesh_builder(&self) -> Self::Output {
        PlaneMeshBuilder {
            plane: *self,
            subdivisions_x: 0,
            subdivisions_z: 0,
        }
    }
}

impl From<Plane3d> for Mesh {
    fn from(value: Plane3d) -> Self {
        value.mesh()
    }
}
