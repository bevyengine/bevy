use bevy_math::{primitives::Ramp, Vec3};
use wgpu::PrimitiveTopology;

use crate::{
    mesh::{Indices, Mesh, Meshable},
    render_asset::RenderAssetUsages,
};

impl Meshable for Ramp {
    type Output = Mesh;

    fn mesh(&self) -> Self::Output {
        let min = -self.half_size;
        let max = self.half_size;

        let top_normal = Vec3::new(0.0, min.z, max.y).normalize_or_zero().to_array();

        // Suppose Y-up right hand, and camera look from +Z to -Z
        let vertices = &[
            // Slope
            ([min.x, max.y, max.z], top_normal, [1.0, 0.0]),
            ([max.x, max.y, max.z], top_normal, [0.0, 0.0]),
            ([max.x, min.y, min.z], top_normal, [0.0, 1.0]),
            ([min.x, min.y, min.z], top_normal, [1.0, 1.0]),
            // Right
            ([max.x, min.y, min.z], [1.0, 0.0, 0.0], [0.0, 0.0]),
            ([max.x, max.y, max.z], [1.0, 0.0, 0.0], [1.0, 1.0]),
            ([max.x, min.y, max.z], [1.0, 0.0, 0.0], [0.0, 1.0]),
            // Left
            ([min.x, min.y, max.z], [-1.0, 0.0, 0.0], [1.0, 0.0]),
            ([min.x, max.y, max.z], [-1.0, 0.0, 0.0], [0.0, 0.0]),
            ([min.x, min.y, min.z], [-1.0, 0.0, 0.0], [1.0, 1.0]),
            // Bottom
            ([max.x, min.y, max.z], [0.0, -1.0, 0.0], [0.0, 0.0]),
            ([min.x, min.y, max.z], [0.0, -1.0, 0.0], [1.0, 0.0]),
            ([min.x, min.y, min.z], [0.0, -1.0, 0.0], [1.0, 1.0]),
            ([max.x, min.y, min.z], [0.0, -1.0, 0.0], [0.0, 1.0]),
            // Front
            ([min.x, max.y, max.z], [0.0, 0.0, 1.0], [0.0, 1.0]),
            ([max.x, max.y, max.z], [0.0, 0.0, 1.0], [1.0, 1.0]),
            ([max.x, min.y, max.z], [0.0, 0.0, 1.0], [1.0, 0.0]),
            ([min.x, min.y, max.z], [0.0, 0.0, 1.0], [0.0, 0.0]),
        ];

        let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
        let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
        let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

        let indices = Indices::U32(vec![
            0, 1, 2, 2, 3, 0, // slope
            4, 5, 6, // right
            7, 8, 9, // left
            10, 11, 12, 12, 13, 10, // bottom
            14, 16, 15, 16, 14, 17, // front
        ]);

        Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        )
        .with_inserted_attribute(Mesh::ATTRIBUTE_POSITION, positions)
        .with_inserted_attribute(Mesh::ATTRIBUTE_NORMAL, normals)
        .with_inserted_attribute(Mesh::ATTRIBUTE_UV_0, uvs)
        .with_inserted_indices(indices)
    }
}

impl From<Ramp> for Mesh {
    fn from(ramp: Ramp) -> Self {
        ramp.mesh()
    }
}
