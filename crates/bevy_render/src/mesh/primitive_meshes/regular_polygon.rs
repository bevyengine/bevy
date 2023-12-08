use super::{Facing, Mesh, MeshFacingExtension, Meshable};
use crate::mesh::Indices;
use bevy_math::primitives::RegularPolygon;
use wgpu::PrimitiveTopology;

#[derive(Clone, Copy, Debug, Default)]
pub struct RegularPolygonMesh {
    pub polygon: RegularPolygon,
    pub facing: Facing,
}

impl MeshFacingExtension for RegularPolygonMesh {
    fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }
}

impl RegularPolygonMesh {
    pub fn build(&self) -> Mesh {
        let mut indices = Vec::with_capacity((self.polygon.sides - 2) * 3);
        let mut positions = Vec::with_capacity(self.polygon.sides);
        let mut normals = Vec::with_capacity(self.polygon.sides);
        let mut uvs = Vec::with_capacity(self.polygon.sides);

        self.build_mesh_data(
            [0.0; 3],
            &mut indices,
            &mut positions,
            &mut normals,
            &mut uvs,
        );

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
            .with_indices(Some(Indices::U32(indices)))
    }

    pub(super) fn build_mesh_data(
        &self,
        translation: [f32; 3],
        indices: &mut Vec<u32>,
        positions: &mut Vec<[f32; 3]>,
        normals: &mut Vec<[f32; 3]>,
        uvs: &mut Vec<[f32; 2]>,
    ) {
        let sides = self.polygon.sides;
        let radius = self.polygon.circumcircle.radius;
        let [trans_x, trans_y, trans_z] = translation;

        let index_offset = positions.len() as u32;
        let facing_coords = self.facing.to_array();
        let normal_sign = self.facing.signum() as f32;
        let step = normal_sign * std::f32::consts::TAU / sides as f32;

        for i in 0..sides {
            let theta = std::f32::consts::FRAC_PI_2 + i as f32 * step;
            let (sin, cos) = theta.sin_cos();
            let x = cos * radius;
            let y = sin * radius;

            let position = match self.facing {
                Facing::X | Facing::NegX => [trans_x, trans_y + y, trans_z - x],
                Facing::Y | Facing::NegY => [trans_x + x, trans_y, trans_z - y],
                Facing::Z | Facing::NegZ => [trans_x + x, trans_y + y, trans_z],
            };

            positions.push(position);
            normals.push(facing_coords);
            uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
        }

        for i in 1..(sides as u32 - 1) {
            indices.extend_from_slice(&[index_offset, index_offset + i, index_offset + i + 1]);
        }
    }
}

impl Meshable for RegularPolygon {
    type Output = RegularPolygonMesh;

    fn mesh(&self) -> Self::Output {
        RegularPolygonMesh {
            polygon: *self,
            ..Default::default()
        }
    }
}

impl From<RegularPolygon> for Mesh {
    fn from(polygon: RegularPolygon) -> Self {
        polygon.mesh().build()
    }
}

impl From<RegularPolygonMesh> for Mesh {
    fn from(polygon: RegularPolygonMesh) -> Self {
        polygon.build()
    }
}
