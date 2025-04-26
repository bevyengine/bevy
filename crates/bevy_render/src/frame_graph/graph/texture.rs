use alloc::sync::Arc;

use crate::frame_graph::{
    AnyFrameGraphResource, AnyFrameGraphResourceDescriptor, FrameGraphTexture, ImportedResource,
    TextureInfo,
};

use super::{GraphResource, GraphResourceDescriptor, ImportToFrameGraph};

impl ImportToFrameGraph for FrameGraphTexture {
    fn import(self: Arc<Self>) -> ImportedResource {
        ImportedResource::Texture(self)
    }
}

impl From<TextureInfo> for AnyFrameGraphResourceDescriptor {
    fn from(value: TextureInfo) -> Self {
        AnyFrameGraphResourceDescriptor::Texture(value)
    }
}

impl GraphResourceDescriptor for TextureInfo {
    type Resource = FrameGraphTexture;
}

impl GraphResource for FrameGraphTexture {
    type Descriptor = TextureInfo;

    fn borrow_resource(res: &AnyFrameGraphResource) -> &Self {
        match res {
            AnyFrameGraphResource::OwnedTexture(res) => res,
            AnyFrameGraphResource::ImportedTexture(res) => res,
            _ => {
                unimplemented!()
            }
        }
    }

    fn get_desc(&self) -> &Self::Descriptor {
        &self.desc
    }
}
