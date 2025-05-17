pub mod bind_group_binding;
pub mod bind_group_entry_binding;
pub mod bind_group_entry_handle;
pub mod bind_group_handle;

pub use bind_group_binding::*;
pub use bind_group_entry_binding::*;
pub use bind_group_entry_handle::*;
pub use bind_group_handle::*;

use crate::{
    frame_graph::{FrameGraph, PassNodeBuilder},
    render_resource::{Buffer, Texture},
};

use super::TextureViewInfo;

pub trait BindGroupResourceHandleHelper {
    fn make_bind_group_resource_handle(
        &self,
        frame_graph: &mut FrameGraph,
    ) -> BindGroupResourceHandle;
}

pub trait BindGroupResourceHelper {
    fn make_binding_group_resource_binding(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindGroupResourceBinding;
}

impl BindGroupResourceHelper for Buffer {
    fn make_binding_group_resource_binding(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindGroupResourceBinding {
        let buffer = pass_node_builder.read_material(self);

        BindingResourceBuffer { buffer, size: None, offest: 0 }.into_binding()
    }
}

impl BindGroupResourceHelper for Texture {
    fn make_binding_group_resource_binding(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindGroupResourceBinding {
        let texture = pass_node_builder.read_material(self);

        BindGroupResourceBinding::TextureView(BindingResourceTextureView {
            texture,
            texture_view_info: TextureViewInfo::default(),
        })
    }
}
