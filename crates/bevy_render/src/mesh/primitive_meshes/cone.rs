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
        let Cone { radius, height } = self.cone;
        let num_vertices = self.resolution * 2 + 1;
        let num_indices = self.resolution * 3;

        let mut positions = Vec::with_capacity(num_vertices as usize);
        let mut normals = Vec::with_capacity(num_vertices as usize);
        let mut uvs = Vec::with_capacity(num_vertices as usize);
        let mut indices = Vec::with_capacity(num_indices as usize);

        // Tip
        positions.push([0.0, self.cone.height / 2.0, 0.0]);

        // The tip doesn't have a singular normal that works correctly.
        // We use an invalid normal here so that it becomes NaN in the fragment shader
        // and doesn't affect the overall shading. This might seem hacky, but it's one of
        // the only ways to get perfectly smooth cones without creases or other shading artefacts.
        //
        // Note that this requires that normals are not normalized in the vertex shader,
        // as that would make the entire triangle invalid and make the cone appear as black.
        normals.push([0.0, 0.0, 0.0]);

        uvs.push([0.5, 0.5]);

        // Lateral surface, i.e. the side of the cone
        let step_theta = std::f32::consts::TAU / self.resolution as f32;
        for segment in 0..=self.resolution {
            let theta = segment as f32 * step_theta;
            let (sin, cos) = theta.sin_cos();

            positions.push([radius * cos, -height / 2.0, radius * sin]);
            normals.push([cos, 0., sin]);
            uvs.push([0.5 + cos * 0.5, 0.5 + sin * 0.5]);
        }

        for j in 0..self.resolution {
            indices.extend_from_slice(&[0, j + 1, j]);
        }

        indices.extend(&[0, positions.len() as u32 - 1, positions.len() as u32 - 2]);

        // Base
        let base = CircleMesh::new(radius, self.resolution as usize).facing_neg_y();
        base.build_mesh_data(
            [0.0, -height / 2.0, 0.0],
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
