use wgpu::{Origin3d, TextureAspect};

use crate::frame_graph::{FrameGraphTexture, ResourceRead, ResourceRef};

#[derive(Clone)]
pub struct TexelCopyTextureInfo {
    pub mip_level: u32,
    pub texture: ResourceRef<FrameGraphTexture, ResourceRead>,
    pub origin: Origin3d,
    pub aspect: TextureAspect,
}
