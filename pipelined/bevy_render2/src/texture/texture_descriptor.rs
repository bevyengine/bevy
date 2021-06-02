use std::num::NonZeroU32;

use crate::texture::TextureViewDimension;

use super::{Extent3d, Texture, TextureDimension, TextureFormat, TextureUsage};

/// Describes a texture
#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash)]
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
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::Rgba8UnormSrgb,
            usage: TextureUsage::SAMPLED | TextureUsage::COPY_DST,
        }
    }
}

#[derive(Hash, Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum StorageTextureAccess {
    /// The texture can only be read in the shader and it must be annotated with `readonly`.
    ///
    /// Example GLSL syntax:
    /// ```cpp,ignore
    /// layout(set=0, binding=0, r32f) readonly uniform image2D myStorageImage;
    /// ```
    ReadOnly,
    /// The texture can only be written in the shader and it must be annotated with `writeonly`.
    ///
    /// Example GLSL syntax:
    /// ```cpp,ignore
    /// layout(set=0, binding=0, r32f) writeonly uniform image2D myStorageImage;
    /// ```
    WriteOnly,
    /// The texture can be both read and written in the shader.
    /// `wgpu::Features::STORAGE_TEXTURE_ACCESS_READ_WRITE` must be enabled to use this access
    /// mode.
    ///
    /// Example GLSL syntax:
    /// ```cpp,ignore
    /// layout(set=0, binding=0, r32f) uniform image2D myStorageImage;
    /// ```
    ReadWrite,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum TextureAspect {
    /// Depth, Stencil, and Color.
    All,
    /// Stencil.
    StencilOnly,
    /// Depth.
    DepthOnly,
}

impl Default for TextureAspect {
    fn default() -> Self {
        Self::All
    }
}

#[derive(Default, Debug, Copy, Clone, Eq, PartialEq)]
pub struct TextureViewDescriptor {
    /// Format of the texture view. At this time, it must be the same as the underlying format of the texture.
    pub format: Option<TextureFormat>,
    /// The dimension of the texture view. For 1D textures, this must be `1D`. For 2D textures it must be one of
    /// `D2`, `D2Array`, `Cube`, and `CubeArray`. For 3D textures it must be `3D`
    pub dimension: Option<TextureViewDimension>,
    /// Aspect of the texture. Color textures must be [`TextureAspect::All`].
    pub aspect: TextureAspect,
    /// Base mip level.
    pub base_mip_level: u32,
    /// Mip level count.
    /// If `Some(count)`, `base_mip_level + count` must be less or equal to underlying texture mip count.
    /// If `None`, considered to include the rest of the mipmap levels, but at least 1 in total.
    pub level_count: Option<NonZeroU32>,
    /// Base array layer.
    pub base_array_layer: u32,
    /// Layer count.
    /// If `Some(count)`, `base_array_layer + count` must be less or equal to the underlying array count.
    /// If `None`, considered to include the rest of the array layers, but at least 1 in total.
    pub array_layer_count: Option<NonZeroU32>,
}
