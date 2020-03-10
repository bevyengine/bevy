use super::TextureDimension;
use crate::asset::Texture;

#[derive(Copy, Clone)]
pub struct TextureDescriptor {
    pub size: wgpu::Extent3d,
    pub array_layer_count: u32,
    pub mip_level_count: u32,
    pub sample_count: u32,
    pub dimension: TextureDimension,
    pub format: wgpu::TextureFormat,
    pub usage: wgpu::TextureUsage,
}

impl From<&Texture> for TextureDescriptor {
    fn from(texture: &Texture) -> Self {
        TextureDescriptor {
            size: wgpu::Extent3d {
                height: texture.height as u32,
                width: texture.width as u32,
                depth: 1,
            },
            array_layer_count: 1,
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8UnormSrgb,
            usage: wgpu::TextureUsage::SAMPLED | wgpu::TextureUsage::COPY_DST,
        }
    }
}

impl From<TextureDescriptor> for wgpu::TextureDescriptor {
    fn from(texture_descriptor: TextureDescriptor) -> Self {
        wgpu::TextureDescriptor {
            size: texture_descriptor.size,
            array_layer_count: texture_descriptor.array_layer_count,
            mip_level_count: texture_descriptor.mip_level_count,
            sample_count: texture_descriptor.sample_count,
            dimension: texture_descriptor.dimension.into(),
            format: texture_descriptor.format,
            usage: texture_descriptor.usage,
        }
    }
}
