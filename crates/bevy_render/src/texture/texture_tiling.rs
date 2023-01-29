use crate::{
    mesh::{Mesh, VertexAttributeValues},
    Resource,
};

/// The way Bevy will attempt to tile your texture
#[derive(Debug)]
pub enum TextureTilingMode {
    /// `TextureTilingMode::Stretch` (equivalent to `TextureTilingMode::Tiles(1.0)`) will take your texture and stretch it across all available space.
    Stretch,
    /// `TextureTilingMode::Tiles(size)` will tile your texture `size` times.
    Tiles(f32),
}

/// The resource used to tile textures.
#[derive(Resource, Debug)]
pub struct TextureTilingSettings(pub (TextureTilingMode, TextureTilingMode));

impl Default for TextureTilingSettings {
    fn default() -> Self {
		Self((TextureTilingMode::Stretch, TextureTilingMode::Stretch))
    }
}

impl TextureTilingSettings {
    pub fn change_tiling_mode(&mut self, new_tiling_mode: TextureTilingSettings) {
        *self = new_tiling_mode;
    }

    pub fn update_mesh_uvs(&self, mesh: &mut Mesh) {
        if let Some(VertexAttributeValues::Float32x2(uvs)) =
            mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
        {
            for uv in uvs {
                if let TextureTilingMode::Tiles(size) = self.0 .0 {
                    uv[0] *= size;
                }
                if let TextureTilingMode::Tiles(size) = self.0 .1 {
                    uv[1] *= size;
                }
            }
        }
    }
}
