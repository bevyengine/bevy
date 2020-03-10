use crate::asset::Texture;

#[derive(Copy, Clone)]
pub struct SamplerDescriptor {
    pub address_mode_u: wgpu::AddressMode,
    pub address_mode_v: wgpu::AddressMode,
    pub address_mode_w: wgpu::AddressMode,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::FilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare_function: wgpu::CompareFunction,
}

impl From<SamplerDescriptor> for wgpu::SamplerDescriptor {
    fn from(sampler_descriptor: SamplerDescriptor) -> Self {
        wgpu::SamplerDescriptor {
            address_mode_u: sampler_descriptor.address_mode_u,
            address_mode_v: sampler_descriptor.address_mode_v,
            address_mode_w: sampler_descriptor.address_mode_w,
            mag_filter: sampler_descriptor.mag_filter,
            min_filter: sampler_descriptor.min_filter,
            mipmap_filter: sampler_descriptor.mipmap_filter,
            lod_min_clamp: sampler_descriptor.lod_min_clamp,
            lod_max_clamp: sampler_descriptor.lod_max_clamp,
            compare_function: sampler_descriptor.compare_function,
        }
    }
}

impl From<&Texture> for SamplerDescriptor {
    fn from(_texture: &Texture) -> Self {
        SamplerDescriptor {
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Nearest,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Nearest,
            lod_min_clamp: -100.0,
            lod_max_clamp: 100.0,
            compare_function: wgpu::CompareFunction::Always,
        }
    }
}
