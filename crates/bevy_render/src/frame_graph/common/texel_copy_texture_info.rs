use wgpu::{Origin3d, TextureAspect};

use crate::frame_graph::{FrameGraphTexture, ResourceRef};

pub struct TexelCopyTextureInfo<ViewType> {
    pub mip_level: u32,
    pub texture: ResourceRef<FrameGraphTexture, ViewType>,
    pub origin: Origin3d,
    pub aspect: TextureAspect,
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
