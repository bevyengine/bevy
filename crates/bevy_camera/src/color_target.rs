use bevy_ecs::prelude::*;
use bevy_math::{UVec2, Vec2};
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

/// Intermediate color target texture that can be used by one or more cameras.
#[derive(Component, Clone, Reflect, PartialEq, Debug)]
#[reflect(Component, PartialEq, Debug, Default)]
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
    pub fn with_size(mut self, size: UVec2) -> Self {
        self.size = size;
        self
    }

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
}

/// Intermediate color target texture that can only be used in a camera.
///
/// Different from [`ColorTarget`]:
/// - `size` can be a factor of camera viewport size.
/// - `format` is not required. If None it is determined by [`crate::Hdr`] and the format of [`crate::RenderTarget`].
/// - `sample_count` isn't here and it is determined by the `Msaa` component.
#[derive(Component, Clone, Reflect, PartialEq, Debug)]
#[reflect(Component, PartialEq, Debug, Default)]
pub struct CameraColorTarget {
    /// Size of the texture.
    pub size: CameraColorTargetSize,
    /// Format of the texture.
    pub format: Option<TextureFormat>,
    /// Allowed usages of the texture.
    pub usage: TextureUsages,
}

#[derive(Clone, Copy, Reflect, PartialEq, Debug)]
#[reflect(PartialEq, Debug)]
pub enum CameraColorTargetSize {
    Factor(Vec2),
    Fixed(UVec2),
}

impl Default for CameraColorTarget {
    fn default() -> Self {
        Self {
            size: CameraColorTargetSize::Factor(Vec2::ONE),
            format: None,
            usage: TextureUsages::RENDER_ATTACHMENT
                | TextureUsages::TEXTURE_BINDING
                | TextureUsages::COPY_SRC,
        }
    }
}

impl CameraColorTarget {
    pub fn with_size(mut self, size: CameraColorTargetSize) -> Self {
        self.size = size;
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

    pub fn with_format(mut self, texture_format: Option<TextureFormat>) -> Self {
        self.format = texture_format;
        self
    }
}
