pub mod bind_group_binding;
pub mod bind_group_entry_binding;
pub mod bind_group_entry_handle;
pub mod bind_group_handle;

pub use bind_group_binding::*;
pub use bind_group_entry_binding::*;
pub use bind_group_entry_handle::*;
pub use bind_group_handle::*;

use crate::frame_graph::{FrameGraph, PassNodeBuilder};

pub trait BindGroupResourceHandleHelper {
    fn make_bind_group_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> BindGroupResourceHandle;
}

pub trait BindGroupTextureViewHandleHelper {
    fn make_bind_group_texture_view_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> BindGroupTextureViewHandle;
}

pub trait BindGroupBufferHandleHelper {
    fn make_bind_group_buffer_handle(&self, frame_graph: &mut FrameGraph) -> BindGroupBufferHandle;
}

pub trait BindGroupResourceBindingHelper {
    fn make_bind_group_resource_binding(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindGroupResourceBinding;
}
