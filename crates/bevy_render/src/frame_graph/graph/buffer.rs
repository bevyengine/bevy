use crate::frame_graph::{
    AnyFrameGraphResource, AnyFrameGraphResourceDescriptor, BufferInfo, FrameGraphBuffer,
};

use super::{GraphResource, GraphResourceDescriptor};

impl From<BufferInfo> for AnyFrameGraphResourceDescriptor {
    fn from(value: BufferInfo) -> Self {
        AnyFrameGraphResourceDescriptor::Buffer(value)
    }
}

impl GraphResourceDescriptor for BufferInfo {
    type Resource = FrameGraphBuffer;
}

impl GraphResource for FrameGraphBuffer {
    type Descriptor = BufferInfo;

    fn borrow_resource(res: &AnyFrameGraphResource) -> &Self {
        match res {
            AnyFrameGraphResource::OwnedBuffer(res) => res,
            AnyFrameGraphResource::ImportedBuffer(res) => res,
            _ => {
                unimplemented!()
            }
        }
    }

    fn get_desc(&self) -> &Self::Descriptor {
        &self.desc
    }
}
