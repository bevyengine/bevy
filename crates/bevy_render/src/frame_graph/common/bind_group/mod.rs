pub mod bind_group_drawing;
pub mod bind_group_entry_handle;
pub mod bind_group_entry_ref;
pub mod bind_group_handle;

pub use bind_group_drawing::*;
pub use bind_group_entry_handle::*;
pub use bind_group_entry_ref::*;
pub use bind_group_handle::*;

use crate::{
    frame_graph::{FrameGraph, PassNodeBuilder, ResourceMaterial},
    render_resource::{DynamicUniformBuffer, UniformBuffer},
};
use encase::{internal::WriteInto, ShaderType};

pub trait BindingResourceHandleHelper {
    fn make_binding_resource_handle(&self, frame_graph: &mut FrameGraph) -> BindingResourceHandle;

    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef;
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
