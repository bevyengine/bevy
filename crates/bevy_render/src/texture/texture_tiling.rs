use crate::mesh::{Mesh, VertexAttributeValues};

/// Update a mesh's UVs so that the applied texture tiles with the given `number_of_tiles`.
pub fn update_mesh_uvs_with_tiling(mesh: &mut Mesh, number_of_tiles: (f32, f32)) {
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for uv in uvs {
            uv[0] *= number_of_tiles.0;
            uv[1] *= number_of_tiles.1;
        }
    }
}

/// Update a mesh's UVs so that the applied texture tiles with the calculated number of tiles,
/// with the size of the mesh, size of the texture (in pixels), and the intended size of the texture in bevy units.
pub fn update_mesh_uvs_with_tiling_by_texture(
    mesh: &mut Mesh,
    mesh_size: f32, // Assumes a square
    texture_size: f32,
    texture_world_space_size: f32,
) {
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for uv in uvs {
            uv[0] *= mesh_size / (texture_size * (texture_world_space_size / texture_size));
            uv[1] *= mesh_size / (texture_size * (texture_world_space_size / texture_size));
        }
    }
}
