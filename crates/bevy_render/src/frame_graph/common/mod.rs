pub mod bind_group;
pub mod color_attachment;
pub mod depth_stencil_attachment;
pub mod render_pass_info;
pub mod sampler_info;
pub mod texture_view;

pub use bind_group::*;
pub use color_attachment::*;
pub use depth_stencil_attachment::*;
pub use render_pass_info::*;
pub use sampler_info::*;
pub use texture_view::*;

use super::{FrameGraph, FrameGraphError, PassNodeBuilder, RenderContext};

pub trait ResourceMaterial {
    type Handle: ResourceHandle;

    fn make_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> Result<Self::Handle, FrameGraphError>;
}

pub trait ResourceHandle {
    type Drawing: ResourceDrawing;

    fn make_resource_drawing(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> Result<Self::Drawing, FrameGraphError>;
}

impl<T: Clone + ResourceDrawing> ResourceHandle for T {
    type Drawing = T;

    fn make_resource_drawing(
        &self,
        _pass_node_builder: &mut PassNodeBuilder,
    ) -> Result<Self::Drawing, FrameGraphError> {
        Ok(self.clone())
    }
}

pub trait ResourceDrawing {
    type Resource;

    fn make_resource<'a>(
        &self,
        render_context: &RenderContext<'a>,
    ) -> Result<Self::Resource, FrameGraphError>;
}
