use bevy_ecs::prelude::*;
use bevy_math::UVec2;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use wgpu_types::{TextureFormat, TextureUsages};

#[derive(Component, Copy, Clone, Reflect, PartialEq, Eq, Hash, Debug)]
#[reflect(Component, PartialEq, Hash, Debug)]
#[relationship(relationship_target=ColorTargetCameras, allow_self_referential)]
pub struct WithColorTarget(pub Entity);

#[derive(Component, Clone, Reflect, PartialEq, Eq, Hash, Debug)]
#[reflect(Component, PartialEq, Hash, Debug)]
#[relationship_target(relationship=WithColorTarget)]
pub struct ColorTargetCameras(Vec<Entity>);

/// The intermediate color target texture (not the [`crate::RenderTarget`]) that can be used for cameras.
#[derive(Component, Clone, Reflect, PartialEq, Eq, Hash, Debug)]
#[reflect(Component, PartialEq, Hash, Debug, Default)]
pub struct ColorTarget {
    /// Size of the texture.
    pub size: UVec2,
    /// Sample count of the multisampled texture if this is larger than 1.
    pub sample_count: u32,
    /// Format of the texture.
    pub format: TextureFormat,
    /// Allowed usages of the texture.
    pub usage: TextureUsages,
}

impl Default for ColorTarget {
    fn default() -> Self {
        Self {
            size: UVec2::new(1280, 720),
            sample_count: 4,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC,
        }
    }
}

impl ColorTarget {
    pub fn with_sample_count(mut self, samples: u32) -> Self {
        self.sample_count = samples;
        self
    }

    pub fn with_usage(mut self, usages: TextureUsages) -> Self {
        self.usage = usages;
        self
    }

    pub fn with_added_usage(mut self, usages: TextureUsages) -> Self {
        self.usage |= usages;
        self
    }

    pub fn with_format(mut self, texture_format: TextureFormat) -> Self {
        self.format = texture_format;
        self
    }

    pub fn with_hdr_format(self) -> Self {
        self.with_format(TextureFormat::Rgba16Float)
    }
}
