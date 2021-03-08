use bevy_asset::{self, Handle};
use bevy_core::Bytes;
use bevy_log::warn;
use bevy_render::{
    impl_render_resource_bytes,
    renderer::{RenderResource, RenderResourceType},
    texture::Texture,
};

const MIN_ROUGHNESS: f32 = 0.089;
const MAX_ROUGHNESS: f32 = 1.0;

#[derive(Clone, Copy, Debug)]
pub enum Roughness {
    PerceptualRoughness(f32),
    Roughness(f32),
}

impl Roughness {
    pub fn perceptual_roughness(roughness: f32) -> Self {
        if roughness < MIN_ROUGHNESS || roughness > MAX_ROUGHNESS {
            warn!(
                "Roughness value {} is not within range [{}, {}]",
                roughness, MIN_ROUGHNESS, MAX_ROUGHNESS
            );
        }
        let perceptual_roughness = roughness.max(MIN_ROUGHNESS).min(MAX_ROUGHNESS);

        Self::PerceptualRoughness(perceptual_roughness)
    }

    pub fn roughness(roughness: f32) -> Self {
        if roughness < MIN_ROUGHNESS || roughness > MAX_ROUGHNESS {
            warn!(
                "Roughness value {} is not within range [{}, {}]",
                roughness, MIN_ROUGHNESS, MAX_ROUGHNESS
            );
        }
        let roughness = roughness.max(MIN_ROUGHNESS).min(MAX_ROUGHNESS);
        Self::Roughness(roughness)
    }

    pub fn to_perceptual_roughness(self) -> f32 {
        // See https://google.github.io/filament/Filament.html#materialsystem/parameterization/remapping
        match self {
            Roughness::PerceptualRoughness(roughness) => roughness,
            Roughness::Roughness(roughness) => roughness.sqrt(),
        }
    }

    pub fn to_roughness(self) -> f32 {
        // See https://google.github.io/filament/Filament.html#materialsystem/parameterization/remapping
        match self {
            Roughness::PerceptualRoughness(roughness) => roughness * roughness,
            Roughness::Roughness(roughness) => roughness,
        }
    }
}

impl From<f32> for Roughness {
    fn from(roughness: f32) -> Self {
        Self::perceptual_roughness(roughness)
    }
}

impl Bytes for Roughness {
    fn write_bytes(&self, buffer: &mut [u8]) {
        self.to_roughness().write_bytes(buffer);
    }

    fn byte_len(&self) -> usize {
        0.byte_len()
    }
}
impl_render_resource_bytes!(Roughness);

#[derive(Copy, Clone, Debug)]
pub struct Metallic(f32);

impl From<f32> for Metallic {
    fn from(metallic: f32) -> Self {
        if metallic < 0.0 || metallic > 1.0 {
            warn!("Metallic value {} is not within range [0.0, 1.0]", metallic);
        }
        Self(metallic)
    }
}

impl Bytes for Metallic {
    fn write_bytes(&self, buffer: &mut [u8]) {
        self.0.write_bytes(buffer);
    }

    fn byte_len(&self) -> usize {
        self.0.byte_len()
    }
}
impl_render_resource_bytes!(Metallic);

pub struct Reflectance(f32);

impl Reflectance {
    fn remap(&self) -> f32 {
        // See https://google.github.io/filament/Filament.html#materialsystem/parameterization/remapping
        0.16 * self.0 * self.0
    }
}

impl From<f32> for Reflectance {
    fn from(reflectance: f32) -> Self {
        if reflectance < 0.0 || reflectance > 1.0 {
            warn!(
                "Reflectance value {} is not within range [0.0, 1.0]",
                reflectance
            );
        }
        Self(reflectance)
    }
}

impl Bytes for Reflectance {
    fn write_bytes(&self, buffer: &mut [u8]) {
        self.remap().write_bytes(buffer);
    }

    fn byte_len(&self) -> usize {
        self.0.byte_len()
    }
}
impl_render_resource_bytes!(Reflectance);
