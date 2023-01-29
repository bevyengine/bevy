use crate::mesh::{Mesh, VertexAttributeValues};

/// The way Bevy will attempt to tile your texture
#[derive(Debug)]
pub enum TextureTilingMode {
    /// `TextureTilingMode::Stretch` (equivalent to `TextureTilingMode::Tiles(1.0)`) will take your texture and stretch it across all available space.
    Stretch,
    /// `TextureTilingMode::Tiles(size)` will tile your texture `size` times.
    Tiles(f32),
}

/// Update a mesh's UVs so that the applied texture tiles as specified.
pub fn update_mesh_uvs_with_tiling(
    mesh: &mut Mesh,
    new_tiling_mode: (TextureTilingMode, TextureTilingMode),
) {
    if let Some(VertexAttributeValues::Float32x2(uvs)) = mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0) {
        for uv in uvs {
            if let TextureTilingMode::Tiles(size) = new_tiling_mode.0 {
                uv[0] *= size;
            }
            if let TextureTilingMode::Tiles(size) = new_tiling_mode.1 {
                uv[1] *= size;
            }
        }
    }
}
