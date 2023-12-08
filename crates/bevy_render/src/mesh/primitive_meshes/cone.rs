use super::{CircleMesh, MeshFacingExtension, Meshable};
use crate::mesh::{Indices, Mesh};
use bevy_math::primitives::Cone;
use wgpu::PrimitiveTopology;

#[derive(Clone, Copy, Debug)]
pub struct ConeMesh {
    pub cone: Cone,
    pub resolution: u32,
}

impl Default for ConeMesh {
    fn default() -> Self {
        Self {
            cone: Cone::default(),
            resolution: 32,
        }
    }
}

impl ConeMesh {
    pub fn resolution(mut self, resolution: usize) -> Self {
        self.resolution = resolution as u32;
        self
    }

    pub fn build(&self) -> Mesh {
        let num_vertices = self.resolution * 2 + 1;
        let num_indices = self.resolution * 3;

        let mut positions = Vec::with_capacity(num_vertices as usize);
        let mut normals = Vec::with_capacity(num_vertices as usize);
        let mut uvs = Vec::with_capacity(num_vertices as usize);
        let mut indices = Vec::with_capacity(num_indices as usize);

        let step_theta = std::f32::consts::TAU / self.resolution as f32;

        // tip

        positions.push([0.0, self.cone.height / 2.0, 0.0]);
        // Invalid normal so that this doesn't affect shading.
        // Requires normalization to be disabled in vertex shader!
        normals.push([0.0, 0.0, 0.0]);
        uvs.push([0.5, 0.5]);

        // lateral surface

        let radius = self.cone.radius;

        for segment in 0..=self.resolution {
            let theta = segment as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            positions.push([radius * cos, -self.cone.height / 2.0, radius * sin]);
            normals.push([cos, 0., sin]);
            uvs.push([0.5 + cos * 0.5, 0.5 + sin * 0.5]);
        }

        for j in 0..self.resolution {
            indices.extend_from_slice(&[0, j + 1, j]);
        }

        indices.extend(&[0, positions.len() as u32 - 1, positions.len() as u32 - 2]);

        // base

        let base = CircleMesh::new(self.cone.radius, self.resolution as usize).facing_neg_y();
        base.build_mesh_data(
            [0.0, -self.cone.height / 2.0, 0.0],
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

impl Meshable for Cone {
    type Output = ConeMesh;

    fn mesh(&self) -> Self::Output {
        ConeMesh {
            cone: *self,
            ..Default::default()
        }
    }
}

impl From<Cone> for Mesh {
    fn from(cone: Cone) -> Self {
        cone.mesh().build()
    }
}

impl From<ConeMesh> for Mesh {
    fn from(cone: ConeMesh) -> Self {
        cone.build()
    }
}
