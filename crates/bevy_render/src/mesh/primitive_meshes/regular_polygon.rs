use super::{Facing, Mesh, Meshable};
use crate::mesh::Indices;
use bevy_math::primitives::RegularPolygon;
use wgpu::PrimitiveTopology;

#[derive(Debug, Default)]
pub struct RegularPolygonMesh {
    pub polygon: RegularPolygon,
    pub facing: Facing,
}

impl RegularPolygonMesh {
    pub const fn facing(mut self, facing: Facing) -> Self {
        self.facing = facing;
        self
    }

    pub const fn facing_x(mut self) -> Self {
        self.facing = Facing::X;
        self
    }

    pub const fn facing_y(mut self) -> Self {
        self.facing = Facing::Y;
        self
    }

    pub const fn facing_z(mut self) -> Self {
        self.facing = Facing::Z;
        self
    }

    pub const fn facing_neg_x(mut self) -> Self {
        self.facing = Facing::NegX;
        self
    }

    pub const fn facing_neg_y(mut self) -> Self {
        self.facing = Facing::NegY;
        self
    }

    pub const fn facing_neg_z(mut self) -> Self {
        self.facing = Facing::NegZ;
        self
    }

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

        debug_assert!(sides > 2, "RegularPolygon requires at least 3 sides.");

        let radius = self.polygon.circumcircle.radius;
        let [trans_x, trans_y, trans_z] = translation;
        let index_offset = positions.len() as u32;
        let normal_sign = self.facing.signum() as f32;
        let step = std::f32::consts::TAU / sides as f32;

        for i in 0..sides {
            let theta = std::f32::consts::FRAC_PI_2 - i as f32 * step;
            let (sin, cos) = theta.sin_cos();

            let (position, normal) = match self.facing {
                Facing::Z | Facing::NegZ => (
                    [trans_x + cos * radius, trans_y + sin * radius, trans_z],
                    [0.0, 0.0, normal_sign],
                ),
                Facing::Y | Facing::NegY => (
                    [trans_x + cos * radius, trans_y, trans_z + sin * radius],
                    [0.0, normal_sign, 0.0],
                ),
                Facing::X | Facing::NegX => (
                    [trans_x, trans_y + cos * radius, trans_z + sin * radius],
                    [normal_sign, 0.0, 0.0],
                ),
            };

            positions.push(position);
            normals.push(normal);
            uvs.push([0.5 * (cos + 1.0), 1.0 - 0.5 * (sin + 1.0)]);
        }

        let winding = if normal_sign < 0.0 { (1, 0) } else { (0, 1) };
        for i in 1..(sides as u32 - 1) {
            indices.extend_from_slice(&[
                index_offset,
                index_offset + i + winding.0,
                index_offset + i + winding.1,
            ]);
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
