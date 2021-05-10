use super::{Extent3d, Texture, TextureDimension, TextureFormat, TextureUsage};

/// Describes a texture
#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct TextureDescriptor {
    pub size: Extent3d,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: TextureDimension,
    pub format: TextureFormat,
    pub usage: TextureUsage,
}

impl From<&Texture> for TextureDescriptor {
    fn from(texture: &Texture) -> Self {
        TextureDescriptor {
            size: texture.size,
            mip_level_count: 1,
            sample_count: 1,
            dimension: texture.dimension,
            format: texture.format,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        }
    }
}

impl Default for TextureDescriptor {
    fn default() -> Self {
        TextureDescriptor {
            size: Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        }
    }
}

#[derive(Hash, Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum StorageTextureAccess {
    /// The texture can only be read in the shader and it must be annotated with `readonly`.
    ///
    /// Example GLSL syntax:
    /// ```cpp,ignore
    /// layout(set=0, binding=0, r32f) readonly uniform image2D myStorageImage;
    /// ```
    ReadOnly,
    /// The texture can only be written in the shader and it must be annotated with `writeonly`.
    ///
    /// Example GLSL syntax:
    /// ```cpp,ignore
    /// layout(set=0, binding=0, r32f) writeonly uniform image2D myStorageImage;
    /// ```
    WriteOnly,
    /// The texture can be both read and written in the shader.
    /// `wgpu::Features::STORAGE_TEXTURE_ACCESS_READ_WRITE` must be enabled to use this access
    /// mode.
    ///
    /// Example GLSL syntax:
    /// ```cpp,ignore
    /// layout(set=0, binding=0, r32f) uniform image2D myStorageImage;
    /// ```
    ReadWrite,
}
