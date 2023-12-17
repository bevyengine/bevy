use super::{Mesh, Meshable};
use crate::mesh::Indices;
use bevy_math::{primitives::Plane3d, Quat, Vec2, Vec3};
use wgpu::PrimitiveTopology;

/// A builder used for creating a [`Mesh`] with a [`Plane3d`] shape.
#[derive(Clone, Copy, Debug)]
pub struct PlaneMesh {
    /// The [`Plane3d`] shape.
    pub plane: Plane3d,
    /// Half the size of the plane mesh.
    pub half_size: Vec2,
}

impl Default for PlaneMesh {
    fn default() -> Self {
        Self {
            plane: Plane3d::default(),
            half_size: Vec2::ONE,
        }
    }
}

impl PlaneMesh {
    /// Creates a new [`PlaneMesh`] from a given normal and size.
    ///
    /// # Panics
    ///
    /// Panics if the given `normal` is zero (or very close to zero), or non-finite.
    #[inline]
    pub fn new(normal: Vec3, size: Vec2) -> Self {
        Self {
            plane: Plane3d::new(normal),
            half_size: 2.0 * size,
        }
    }

    /// Sets the size of the plane mesh.
    #[inline]
    pub fn size(mut self, size: Vec2) -> Self {
        self.half_size = size / 2.0;
        self
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        let rotation = Quat::from_rotation_arc(Vec3::Y, *self.plane.normal);
        let positions = vec![
            rotation * Vec3::new(self.half_size.x, 0.0, -self.half_size.y),
            rotation * Vec3::new(-self.half_size.x, 0.0, -self.half_size.y),
            rotation * Vec3::new(-self.half_size.x, 0.0, self.half_size.y),
            rotation * Vec3::new(self.half_size.x, 0.0, self.half_size.y),
        ];

        let normals = vec![self.plane.normal.to_array(); 4];
        let uvs = vec![[1.0, 0.0], [0.0, 0.0], [0.0, 1.0], [1.0, 1.0]];
        let indices = Indices::U32(vec![0, 1, 2, 0, 2, 3]);

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_indices(Some(indices))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Plane3d {
    type Output = PlaneMesh;

    fn mesh(&self) -> Self::Output {
        PlaneMesh {
            plane: *self,
            ..Default::default()
        }
    }
}

impl From<Plane3d> for Mesh {
    fn from(plane: Plane3d) -> Self {
        plane.mesh().build()
    }
}

impl From<PlaneMesh> for Mesh {
    fn from(plane: PlaneMesh) -> Self {
        plane.build()
    }
}
