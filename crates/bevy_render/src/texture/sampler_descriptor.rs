use crate::pipeline::CompareFunction;
use std::num::NonZeroU8;

/// Describes a sampler
#[derive(Debug, Copy, Clone)]
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
    pub anisotropy_clamp: Option<NonZeroU8>,
    pub border_color: Option<SamplerBorderColor>,
}

impl SamplerDescriptor {
    /// Sets the address mode for all dimensions of the sampler descriptor.
    pub fn set_address_mode(&mut self, address_mode: AddressMode) {
        self.address_mode_u = address_mode;
        self.address_mode_v = address_mode;
        self.address_mode_w = address_mode;
    }
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
            border_color: None,
        }
    }
}

/// How edges should be handled in texture addressing.
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

/// Texel mixing mode when sampling between texels.
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

#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub enum SamplerBorderColor {
    TransparentBlack,
    OpaqueBlack,
    OpaqueWhite,
}
