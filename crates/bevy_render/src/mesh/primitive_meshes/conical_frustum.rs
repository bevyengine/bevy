use super::Meshable;
use crate::mesh::{
    primitive_meshes::{CircleMesh, MeshFacingExtension},
    Indices, Mesh,
};
use bevy_math::{primitives::ConicalFrustum, Vec3};
use wgpu::PrimitiveTopology;

#[derive(Clone, Copy, Debug)]
pub struct ConicalFrustumMesh {
    pub frustum: ConicalFrustum,
    pub resolution: u32,
    pub segments: u32,
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
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    pub fn segments(mut self, segments: u32) -> Self {
        self.segments = segments;
        self
    }

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

        let mut positions = Vec::with_capacity(num_vertices as usize);
        let mut normals = Vec::with_capacity(num_vertices as usize);
        let mut uvs = Vec::with_capacity(num_vertices as usize);
        let mut indices = Vec::with_capacity(num_indices as usize);

        let step_theta = std::f32::consts::TAU / self.resolution as f32;
        let step_y = height / self.segments as f32;
        let step_radius = (radius_top - radius_bottom) / self.segments as f32;

        // rings

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

        // barrel skin

        for i in 0..self.segments {
            let ring = i * (self.resolution + 1);
            let next_ring = (i + 1) * (self.resolution + 1);

            for j in 0..self.resolution {
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

        let top = CircleMesh::new(self.frustum.radius_top, self.resolution as usize).facing_y();
        top.build_mesh_data(
            [0.0, self.frustum.height / 2.0, 0.0],
            &mut indices,
            &mut positions,
            &mut normals,
            &mut uvs,
        );

        let bottom =
            CircleMesh::new(self.frustum.radius_bottom, self.resolution as usize).facing_neg_y();
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
