use super::Meshable;
use crate::mesh::{primitive_meshes::CircleMesh, Indices, Mesh};
use bevy_math::primitives::Cylinder;
use wgpu::PrimitiveTopology;

pub struct CylinderMesh {
    pub cylinder: Cylinder,
    pub resolution: u32,
    pub segments: u32,
}

impl CylinderMesh {
    pub fn resolution(mut self, resolution: u32) -> Self {
        self.resolution = resolution;
        self
    }

    pub fn segments(mut self, segments: u32) -> Self {
        self.segments = segments;
        self
    }

    pub fn build(&self) -> Mesh {
        let resolution = self.resolution;
        let segments = self.segments;

        debug_assert!(resolution > 2);
        debug_assert!(segments > 0);

        let num_rings = segments + 1;
        let num_vertices = resolution * 2 + num_rings * (resolution + 1);
        let num_faces = resolution * (num_rings - 2);
        let num_indices = (2 * num_faces + 2 * (resolution - 1) * 2) * 3;

        let mut positions = Vec::with_capacity(num_vertices as usize);
        let mut normals = Vec::with_capacity(num_vertices as usize);
        let mut uvs = Vec::with_capacity(num_vertices as usize);
        let mut indices = Vec::with_capacity(num_indices as usize);

        let step_theta = std::f32::consts::TAU / resolution as f32;
        let step_y = 2.0 * self.cylinder.half_height / segments as f32;

        // rings

        for ring in 0..num_rings {
            let y = -self.cylinder.half_height + ring as f32 * step_y;

            for segment in 0..=resolution {
                let theta = segment as f32 * step_theta;
                let (sin, cos) = theta.sin_cos();

                positions.push([self.cylinder.radius * cos, y, self.cylinder.radius * sin]);
                normals.push([cos, 0., sin]);
                uvs.push([
                    segment as f32 / resolution as f32,
                    ring as f32 / segments as f32,
                ]);
            }
        }

        // barrel skin

        for i in 0..segments {
            let ring = i * (resolution + 1);
            let next_ring = (i + 1) * (resolution + 1);

            for j in 0..resolution {
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

        // Top and bottom
        let base = CircleMesh::new(self.cylinder.radius, self.resolution as usize).facing_y();
        base.build_mesh_data(
            [0.0, self.cylinder.half_height, 0.0],
            &mut indices,
            &mut positions,
            &mut normals,
            &mut uvs,
        );
        base.facing_neg_y().build_mesh_data(
            [0.0, -self.cylinder.half_height, 0.0],
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

impl Meshable for Cylinder {
    type Output = CylinderMesh;

    fn mesh(&self) -> Self::Output {
        CylinderMesh {
            cylinder: *self,
            resolution: 16,
            segments: 1,
        }
    }
}

impl From<Cylinder> for Mesh {
    fn from(cylinder: Cylinder) -> Self {
        cylinder.mesh().build()
    }
}

impl From<CylinderMesh> for Mesh {
    fn from(cylinder: CylinderMesh) -> Self {
        cylinder.build()
    }
}
