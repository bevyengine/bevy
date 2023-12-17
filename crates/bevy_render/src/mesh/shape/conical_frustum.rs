use super::{CircleMesh, MeshFacingExtension, Meshable};
use crate::mesh::{Indices, Mesh};
use bevy_math::{primitives::ConicalFrustum, Vec3};
use wgpu::PrimitiveTopology;

/// A builder used for creating a [`Mesh`] with a [`ConicalFrustum`] shape.
#[derive(Clone, Copy, Debug)]
pub struct ConicalFrustumMesh {
    /// The [`ConicalFrustum`] shape.
    pub frustum: ConicalFrustum,
    /// The number of vertices used for the top and bottom of the conical frustum.
    /// The default is `32`.
    pub resolution: usize,
    /// The number of horizontal lines subdividing the lateral surface of the conical frustum.
    /// The default is `1`.
    pub segments: usize,
}

impl Default for ConicalFrustumMesh {
    fn default() -> Self {
        Self {
            frustum: ConicalFrustum::default(),
            resolution: 32,
            segments: 1,
        }
    }
}

impl ConicalFrustumMesh {
    /// Creates a new [`ConicalFrustumMesh`] from the given top and bottom radii, a height,
    /// and a resolution used for the top and bottom.
    #[inline]
    pub const fn new(radius_top: f32, radius_bottom: f32, height: f32, resolution: usize) -> Self {
        Self {
            frustum: ConicalFrustum {
                radius_top,
                radius_bottom,
                height,
            },
            resolution,
            segments: 1,
        }
    }

    /// Sets the number of vertices used for the top and bottom of the conical frustum.
    #[inline]
    pub const fn resolution(mut self, resolution: usize) -> Self {
        self.resolution = resolution;
        self
    }

    /// Sets the number of horizontal lines subdividing the lateral surface of the conical frustum.
    #[inline]
    pub const fn segments(mut self, segments: usize) -> Self {
        self.segments = segments;
        self
    }

    /// Builds a [`Mesh`] based on the configuration in `self`.
    pub fn build(&self) -> Mesh {
        debug_assert!(self.resolution > 2);
        debug_assert!(self.segments > 0);

        let ConicalFrustum {
            radius_top,
            radius_bottom,
            height,
        } = self.frustum;

        let num_rings = self.segments + 1;
        let num_vertices = self.resolution * 2 + num_rings * (self.resolution + 1);
        let num_faces = self.resolution * (num_rings - 2);
        let num_indices = (2 * num_faces + 2 * (self.resolution - 1) * 2) * 3;

        let mut positions = Vec::with_capacity(num_vertices);
        let mut normals = Vec::with_capacity(num_vertices);
        let mut uvs = Vec::with_capacity(num_vertices);
        let mut indices = Vec::with_capacity(num_indices);

        let step_theta = std::f32::consts::TAU / self.resolution as f32;
        let step_y = height / self.segments as f32;
        let step_radius = (radius_top - radius_bottom) / self.segments as f32;

        // Rings
        for ring in 0..num_rings {
            let y = -height / 2.0 + ring as f32 * step_y;
            let radius = radius_bottom + ring as f32 * step_radius;

            for segment in 0..=self.resolution {
                let theta = segment as f32 * step_theta;
                let (sin, cos) = theta.sin_cos();

                positions.push([radius * cos, y, radius * sin]);
                normals.push(
                    Vec3::new(cos, (radius_bottom - radius_top) / height, sin)
                        .normalize()
                        .to_array(),
                );
                uvs.push([
                    segment as f32 / self.resolution as f32,
                    ring as f32 / self.segments as f32,
                ]);
            }
        }

        // Lateral surface
        for i in 0..self.segments as u32 {
            let ring = i * (self.resolution + 1) as u32;
            let next_ring = (i + 1) * (self.resolution + 1) as u32;

            for j in 0..self.resolution as u32 {
                indices.extend_from_slice(&[
                    ring + j,
                    next_ring + j,
                    ring + j + 1,
                    next_ring + j,
                    next_ring + j + 1,
                    ring + j + 1,
                ]);
            }
        }

        let top = CircleMesh::new(self.frustum.radius_top, self.resolution).facing_y();
        top.build_mesh_data(
            [0.0, self.frustum.height / 2.0, 0.0],
            &mut indices,
            &mut positions,
            &mut normals,
            &mut uvs,
        );

        let bottom = CircleMesh::new(self.frustum.radius_bottom, self.resolution).facing_neg_y();
        bottom.build_mesh_data(
            [0.0, -self.frustum.height / 2.0, 0.0],
            &mut indices,
            &mut positions,
            &mut normals,
            &mut uvs,
        );

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_indices(Some(Indices::U32(indices)))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}

impl Meshable for ConicalFrustum {
    type Output = ConicalFrustumMesh;

    fn mesh(&self) -> Self::Output {
        ConicalFrustumMesh {
            frustum: *self,
            ..Default::default()
        }
    }
}

impl From<ConicalFrustum> for Mesh {
    fn from(frustum: ConicalFrustum) -> Self {
        frustum.mesh().build()
    }
}

impl From<ConicalFrustumMesh> for Mesh {
    fn from(frustum: ConicalFrustumMesh) -> Self {
        frustum.build()
    }
}
