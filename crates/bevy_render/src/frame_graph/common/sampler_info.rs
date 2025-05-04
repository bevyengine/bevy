use alloc::borrow::Cow;
use bevy_image::ImageSamplerDescriptor;

#[derive(Debug, Clone)]
pub struct SamplerInfo {
    pub label: Option<Cow<'static, str>>,
    pub address_mode_u: wgpu::AddressMode,
    pub address_mode_v: wgpu::AddressMode,
    pub address_mode_w: wgpu::AddressMode,
    pub mag_filter: wgpu::FilterMode,
    pub min_filter: wgpu::FilterMode,
    pub mipmap_filter: wgpu::FilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare: Option<wgpu::CompareFunction>,
    pub anisotropy_clamp: u16,
    pub border_color: Option<wgpu::SamplerBorderColor>,
}

impl Default for SamplerInfo {
    fn default() -> Self {
        Self {
            label: Default::default(),
            address_mode_u: Default::default(),
            address_mode_v: Default::default(),
            address_mode_w: Default::default(),
            mag_filter: Default::default(),
            min_filter: Default::default(),
            mipmap_filter: Default::default(),
            lod_min_clamp: 0.0,
            lod_max_clamp: 32.0,
            compare: None,
            anisotropy_clamp: 1,
            border_color: None,
        }
    }
}

impl SamplerInfo {
    pub fn new_image_sampler_descriptor(desc: &ImageSamplerDescriptor) -> Self {
        SamplerInfo {
            label: desc.label.as_ref().map(|label| label.clone().into()),
            address_mode_u: desc.address_mode_u.into(),
            address_mode_v: desc.address_mode_v.into(),
            address_mode_w: desc.address_mode_w.into(),
            mag_filter: desc.mag_filter.into(),
            min_filter: desc.min_filter.into(),
            mipmap_filter: desc.mipmap_filter.into(),
            lod_min_clamp: desc.lod_min_clamp,
            lod_max_clamp: desc.lod_max_clamp,
            compare: desc.compare.map(Into::into),
            anisotropy_clamp: desc.anisotropy_clamp,
            border_color: desc.border_color.map(Into::into),
        }
    }

    pub fn get_sample_desc(&self) -> wgpu::SamplerDescriptor {
        wgpu::SamplerDescriptor {
            label: self.label.as_deref(),
            address_mode_u: self.address_mode_u,
            address_mode_v: self.address_mode_v,
            address_mode_w: self.address_mode_w,
            mag_filter: self.mag_filter,
            min_filter: self.min_filter,
            mipmap_filter: self.mipmap_filter,
            lod_min_clamp: self.lod_min_clamp,
            lod_max_clamp: self.lod_max_clamp,
            compare: self.compare,
            anisotropy_clamp: self.anisotropy_clamp,
            border_color: self.border_color,
        }
    }
}