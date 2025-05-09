pub mod bind_group_drawing;
pub mod bind_group_entry_handle;
pub mod bind_group_entry_ref;
pub mod bind_group_handle;

pub use bind_group_drawing::*;
pub use bind_group_entry_handle::*;
pub use bind_group_entry_ref::*;
pub use bind_group_handle::*;

use crate::{
    frame_graph::{
        FrameGraph, FrameGraphBuffer, FrameGraphTexture, GraphResourceNodeHandle, PassNodeBuilder,
        ResourceMaterial,
    },
    render_resource::{Buffer, DynamicUniformBuffer, StorageBuffer, Texture, UniformBuffer},
    texture::GpuImage,
};
use encase::{internal::WriteInto, ShaderType};

use super::{SamplerInfo, TextureViewInfo};

pub trait BindingResourceHandleHelper {
    fn make_binding_resource_handle(&self, frame_graph: &mut FrameGraph) -> BindingResourceHandle;

    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef;
}

impl BindingResourceHandleHelper for SamplerInfo {
    fn make_binding_resource_handle(&self, _frame_graph: &mut FrameGraph) -> BindingResourceHandle {
        BindingResourceHandle::Sampler(self.clone())
    }

    fn make_binding_resource_ref(
        &self,
        _pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        BindingResourceRef::Sampler(self.clone())
    }
}

impl<'a> BindingResourceHandleHelper
    for (
        &'a GraphResourceNodeHandle<FrameGraphTexture>,
        &'a TextureViewInfo,
    )
{
    fn make_binding_resource_handle(&self, _frame_graph: &mut FrameGraph) -> BindingResourceHandle {
        BindingResourceHandle::TextureView {
            texture: self.0.clone(),
            texture_view_info: self.1.clone(),
        }
    }

    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        let texture = pass_node_builder.read(self.0.clone());

        BindingResourceRef::TextureView {
            texture,
            texture_view_info: self.1.clone(),
        }
    }
}

impl BindingResourceHandleHelper for GraphResourceNodeHandle<FrameGraphTexture> {
    fn make_binding_resource_handle(&self, _frame_graph: &mut FrameGraph) -> BindingResourceHandle {
        BindingResourceHandle::TextureView {
            texture: self.clone(),
            texture_view_info: TextureViewInfo::default(),
        }
    }

    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        let texture = pass_node_builder.read(self.clone());

        BindingResourceRef::TextureView {
            texture,
            texture_view_info: TextureViewInfo::default(),
        }
    }
}

impl BindingResourceHandleHelper for GraphResourceNodeHandle<FrameGraphBuffer> {
    fn make_binding_resource_handle(&self, _frame_graph: &mut FrameGraph) -> BindingResourceHandle {
        BindingResourceHandle::Buffer {
            buffer: self.clone(),
            size: None,
        }
    }

    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        let buffer = pass_node_builder.read(self.clone());
        BindingResourceRef::Buffer { buffer, size: None }
    }
}

impl BindingResourceHandleHelper for Buffer {
    fn make_binding_resource_handle(&self, frame_graph: &mut FrameGraph) -> BindingResourceHandle {
        let buffer = self.make_resource_handle(frame_graph);
        BindingResourceHandle::Buffer { buffer, size: None }
    }

    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        let buffer = pass_node_builder.import_and_read_buffer(self);
        BindingResourceRef::Buffer { buffer, size: None }
    }
}

impl BindingResourceHandleHelper for Texture {
    fn make_binding_resource_handle(&self, frame_graph: &mut FrameGraph) -> BindingResourceHandle {
        let texture = self.make_resource_handle(frame_graph);
        BindingResourceHandle::TextureView {
            texture,
            texture_view_info: TextureViewInfo::default(),
        }
    }

    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        let texture = pass_node_builder.import_and_read_texture(&self);

        BindingResourceRef::TextureView {
            texture,
            texture_view_info: TextureViewInfo::default(),
        }
    }
}

impl BindingResourceHandleHelper for GpuImage {
    fn make_binding_resource_handle(&self, frame_graph: &mut FrameGraph) -> BindingResourceHandle {
        let texture = self.texture.make_resource_handle(frame_graph);
        BindingResourceHandle::TextureView {
            texture,
            texture_view_info: TextureViewInfo::default(),
        }
    }

    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        let texture = pass_node_builder.import_and_read_texture(&self.texture);

        BindingResourceRef::TextureView {
            texture,
            texture_view_info: TextureViewInfo::default(),
        }
    }
}

impl<T: ShaderType + WriteInto> BindingResourceHandleHelper for StorageBuffer<T> {
    fn make_binding_resource_handle(&self, frame_graph: &mut FrameGraph) -> BindingResourceHandle {
        let buffer = self.buffer().expect("buffer must have");
        let handle = buffer.make_resource_handle(frame_graph);

        let size = T::min_size();

        BindingResourceHandle::Buffer {
            buffer: handle,
            size: Some(size),
        }
    }

    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        let buffer = self.buffer().expect("buffer must have");
        let buffer_ref = pass_node_builder.import_and_read_buffer(buffer);

        let size = T::min_size();
        BindingResourceRef::Buffer {
            buffer: buffer_ref,
            size: Some(size),
        }
    }
}

impl<T: ShaderType + WriteInto> BindingResourceHandleHelper for UniformBuffer<T> {
    fn make_binding_resource_handle(&self, frame_graph: &mut FrameGraph) -> BindingResourceHandle {
        let buffer = self.buffer().expect("buffer must have");
        let handle = buffer.make_resource_handle(frame_graph);

        let size = T::min_size();

        BindingResourceHandle::Buffer {
            buffer: handle,
            size: Some(size),
        }
    }

    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        let buffer = self.buffer().expect("buffer must have");
        let buffer_ref = pass_node_builder.import_and_read_buffer(buffer);

        let size = T::min_size();
        BindingResourceRef::Buffer {
            buffer: buffer_ref,
            size: Some(size),
        }
    }
}

impl<T: ShaderType + WriteInto> BindingResourceHandleHelper for DynamicUniformBuffer<T> {
    fn make_binding_resource_handle(&self, frame_graph: &mut FrameGraph) -> BindingResourceHandle {
        let buffer = self.buffer().expect("buffer must have");
        let handle = buffer.make_resource_handle(frame_graph);

        let size = T::min_size();

        BindingResourceHandle::Buffer {
            buffer: handle,
            size: Some(size),
        }
    }

    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        let buffer = self.buffer().expect("buffer must have");
        let buffer_ref = pass_node_builder.import_and_read_buffer(buffer);

        let size = T::min_size();
        BindingResourceRef::Buffer {
            buffer: buffer_ref,
            size: Some(size),
        }
    }
}
