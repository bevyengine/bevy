use bevy_math::{primitives::Plane3d, Dir3, Quat, Vec2, Vec3};
use wgpu::PrimitiveTopology;

use crate::{
    mesh::{Indices, Mesh, Meshable},
    render_asset::RenderAssetUsages,
};

/// A builder used for creating a [`Mesh`] with a [`Plane3d`] shape.
#[derive(Clone, Copy, Debug)]
pub struct PlaneMeshBuilder {
    /// The [`Plane3d`] shape.
    pub plane: Plane3d,
    /// Half the size of the plane mesh.
    pub half_size: Vec2,
}

impl Default for PlaneMeshBuilder {
    fn default() -> Self {
        Self {
            plane: Plane3d::default(),
            half_size: Vec2::ONE,
        }
    }
}

impl PlaneMeshBuilder {
    /// Creates a new [`PlaneMeshBuilder`] from a given normal and size.
    #[inline]
    pub fn new(normal: Dir3, size: Vec2) -> Self {
        Self {
            plane: Plane3d { normal },
            half_size: size / 2.0,
        }
    }

    /// Creates a new [`PlaneMeshBuilder`] from the given size, with the normal pointing upwards.
    #[inline]
    pub fn from_size(size: Vec2) -> Self {
        Self {
            half_size: size / 2.0,
            ..Default::default()
        }
    }

    /// Creates a new [`PlaneMeshBuilder`] from the given length, with the normal pointing upwards,
    /// and the resulting [`PlaneMeshBuilder`] being a square.
    #[inline]
    pub fn from_length(length: f32) -> Self {
        Self {
            half_size: Vec2::splat(length) / 2.0,
            ..Default::default()
        }
    }

    /// Sets the normal of the plane, aka the direction the plane is facing.
    #[inline]
    #[doc(alias = "facing")]
    pub fn normal(mut self, normal: Dir3) -> Self {
        self.plane = Plane3d { normal };
        self
    }

    /// Sets the size of the plane mesh.
    #[inline]
    pub fn size(mut self, width: f32, height: f32) -> Self {
        self.half_size = Vec2::new(width, height) / 2.0;
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

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_indices(indices)
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for Plane3d {
    type Output = PlaneMeshBuilder;

    fn mesh(&self) -> Self::Output {
        PlaneMeshBuilder {
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

impl From<PlaneMeshBuilder> for Mesh {
    fn from(plane: PlaneMeshBuilder) -> Self {
        plane.build()
    }
}
