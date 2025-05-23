use wgpu::{Origin3d, TexelCopyBufferLayout, TextureAspect};

use crate::frame_graph::{TransientBuffer, TransientTexture, Ref};

pub struct TexelCopyBufferInfo<ViewType> {
    pub buffer: Ref<TransientBuffer, ViewType>,
    pub layout: TexelCopyBufferLayout,
}

impl<ViewType> Clone for TexelCopyBufferInfo<ViewType> {
    fn clone(&self) -> Self {
        Self {
            buffer: self.buffer.clone(),
            layout: self.layout.clone(),
        }
    }
}

pub struct TexelCopyTextureInfo<ViewType> {
    pub mip_level: u32,
    pub texture: Ref<TransientTexture, ViewType>,
    pub origin: Origin3d,
    pub aspect: TextureAspect,
}

impl<ViewType> TexelCopyTextureInfo<ViewType> {
    pub fn new(texture: Ref<TransientTexture, ViewType>) -> Self {
        TexelCopyTextureInfo {
            mip_level: 0,
            texture,
            origin: Origin3d::ZERO,
            aspect: TextureAspect::All,
        }
    }
}

impl<ViewType> Clone for TexelCopyTextureInfo<ViewType> {
    fn clone(&self) -> Self {
        Self {
            mip_level: self.mip_level,
            texture: self.texture.clone(),
            origin: self.origin,
            aspect: self.aspect,
        }
    }
}
