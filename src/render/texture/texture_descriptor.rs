use super::{Extent3d, TextureDimension, TextureFormat};
use crate::{asset::Texture, render::texture::TextureUsage};

#[derive(Copy, Clone)]
pub struct TextureDescriptor {
    pub size: Extent3d,
    pub array_layer_count: u32,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: TextureDimension,
    pub format: TextureFormat,
    pub usage: TextureUsage,
}

impl From<&Texture> for TextureDescriptor {
    fn from(texture: &Texture) -> Self {
        TextureDescriptor {
            size: Extent3d {
                height: texture.height as u32,
                width: texture.width as u32,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        }
    }
}
