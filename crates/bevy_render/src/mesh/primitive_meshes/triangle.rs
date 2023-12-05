use super::{Mesh, Meshable};
use crate::mesh::Indices;
use bevy_math::{
    primitives::{Triangle2d, WindingOrder},
    Vec2,
};
use wgpu::PrimitiveTopology;

impl Meshable for Triangle2d {
    type Output = Mesh;

    fn mesh(&self) -> Mesh {
        let [a, b, c] = self.vertices;
        let max = a.min(b).min(c).abs().max(a.max(b).max(c)) * Vec2::new(1.0, -1.0);
        let [norm_a, norm_b, norm_c] = [(a) / max, (b) / max, (c) / max];
        let vertices = [
            (a.extend(0.0), [0.0, 0.0, 1.0], norm_a / 2.0 + 0.5),
            (b.extend(0.0), [0.0, 0.0, 1.0], norm_b / 2.0 + 0.5),
            (c.extend(0.0), [0.0, 0.0, 1.0], norm_c / 2.0 + 0.5),
        ];

        let indices = if self.winding_order() == WindingOrder::CounterClockwise {
            Indices::U32(vec![0, 1, 2])
        } else {
            Indices::U32(vec![0, 2, 1])
        };

        let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
        let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
        let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

        Mesh::new(PrimitiveTopology::TriangleList)
            .with_indices(Some(indices))
            .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
            .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
            .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
    }
}
impl From<Triangle2d> for Mesh {
    fn from(triangle: Triangle2d) -> Self {
        triangle.mesh()
    }
}
