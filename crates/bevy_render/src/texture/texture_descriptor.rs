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
                depth: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        }
    }
}
