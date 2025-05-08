pub mod bind_group;
pub mod color_attachment;
pub mod depth_stencil_attachment;
pub mod render_pass_info;
pub mod sampler_info;
pub mod texel_copy_texture_info;
pub mod texture_view;
pub mod compute_pass_info;

pub use bind_group::*;
pub use color_attachment::*;
pub use depth_stencil_attachment::*;
pub use render_pass_info::*;
pub use sampler_info::*;
pub use texel_copy_texture_info::*;
pub use texture_view::*;
pub use compute_pass_info::*;

use crate::render_resource::{Buffer, Texture};

use super::{
    FrameGraph, FrameGraphBuffer, FrameGraphError, FrameGraphTexture, GraphResourceNodeHandle,
    PassNodeBuilder, RenderContext,
};

pub trait ResourceMaterial {
    type Handle;

    fn make_resource_handle(&self, frame_graph: &mut FrameGraph) -> Self::Handle;
}

impl ResourceMaterial for Buffer {
    type Handle = GraphResourceNodeHandle<FrameGraphBuffer>;

    fn make_resource_handle(&self, frame_graph: &mut FrameGraph) -> Self::Handle {
        let key = format!("buffer_{:?}", self.id());
        let buffer = FrameGraphBuffer::new_arc_with_buffer(self);
        let handle = frame_graph.import(&key, buffer);
        handle
    }
}

impl ResourceMaterial for Texture {
    type Handle = GraphResourceNodeHandle<FrameGraphTexture>;

    fn make_resource_handle(&self, frame_graph: &mut FrameGraph) -> Self::Handle {
        let key = format!("texture_{:?}", self.id());
        let texture = FrameGraphTexture::new_arc_with_texture(self);
        let handle = frame_graph.import(&key, texture);
        handle
    }
}

pub trait ResourceHandle {
    type Drawing;

    fn make_resource_drawing(&self, pass_node_builder: &mut PassNodeBuilder) -> Self::Drawing;
}

impl<T: Clone + ResourceDrawing> ResourceHandle for T {
    type Drawing = T;

    fn make_resource_drawing(&self, _pass_node_builder: &mut PassNodeBuilder) -> Self::Drawing {
        self.clone()
    }
}

pub trait ResourceDrawing {
    type Resource;

    fn make_resource<'a>(
        &self,
        render_context: &RenderContext<'a>,
    ) -> Result<Self::Resource, FrameGraphError>;
}
