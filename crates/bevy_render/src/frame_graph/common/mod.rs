pub mod bind_group;
pub mod color_attachment;
pub mod compute_pass_info;
pub mod depth_stencil_attachment;
pub mod render_pass_info;
pub mod resource_meta;
pub mod texel_copy_texture_info;
pub mod texture_view;

pub use bind_group::*;
pub use color_attachment::*;
pub use compute_pass_info::*;
pub use depth_stencil_attachment::*;
pub use render_pass_info::*;
pub use resource_meta::*;
pub use texel_copy_texture_info::*;
pub use texture_view::*;

use crate::render_resource::{Buffer, Texture};

use super::{
    FrameGraph, FrameGraphBuffer, FrameGraphError, FrameGraphTexture, GraphResource,
    GraphResourceNodeHandle, PassNodeBuilder, RenderContext,
};

pub trait ResourceMaterial {
    type ResourceType: GraphResource;

    fn make_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> GraphResourceNodeHandle<Self::ResourceType>;
}

impl ResourceMaterial for Buffer {
    type ResourceType = FrameGraphBuffer;

    fn make_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> GraphResourceNodeHandle<FrameGraphBuffer> {
        let key = format!("buffer_{:?}", self.id());
        let buffer = FrameGraphBuffer::new_arc_with_buffer(self);
        let handle = frame_graph.import(&key, buffer);
        handle
    }
}

impl ResourceMaterial for Texture {
    type ResourceType = FrameGraphTexture;

    fn make_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> GraphResourceNodeHandle<FrameGraphTexture> {
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
