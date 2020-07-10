use super::Texture;
use crate::pipeline::state_descriptors::CompareFunction;

#[derive(Copy, Clone)]
pub struct SamplerDescriptor {
    pub address_mode_u: AddressMode,
    pub address_mode_v: AddressMode,
    pub address_mode_w: AddressMode,
    pub mag_filter: FilterMode,
    pub min_filter: FilterMode,
    pub mipmap_filter: FilterMode,
    pub lod_min_clamp: f32,
    pub lod_max_clamp: f32,
    pub compare_function: Option<CompareFunction>,
    pub anisotropy_clamp: Option<u8>,
}

impl Default for SamplerDescriptor {
    fn default() -> Self {
        SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: std::f32::MAX,
            compare_function: None,
            anisotropy_clamp: None,
        }
    }
}

impl From<&Texture> for SamplerDescriptor {
    fn from(_texture: &Texture) -> Self {
        SamplerDescriptor {
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Nearest,
            min_filter: FilterMode::Linear,
            mipmap_filter: FilterMode::Nearest,
            lod_min_clamp: 0.0,
            lod_max_clamp: std::f32::MAX,
            compare_function: None,
            anisotropy_clamp: None,
        }
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum AddressMode {
    ClampToEdge = 0,
    Repeat = 1,
    MirrorRepeat = 2,
}

impl Default for AddressMode {
    fn default() -> Self {
        AddressMode::ClampToEdge
    }
}

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum FilterMode {
    Nearest = 0,
    Linear = 1,
}

impl Default for FilterMode {
    fn default() -> Self {
        FilterMode::Nearest
    }
}
