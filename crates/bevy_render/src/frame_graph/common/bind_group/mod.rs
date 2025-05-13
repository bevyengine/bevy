pub mod bind_group_drawing;
pub mod bind_group_entry_handle;
pub mod bind_group_entry_ref;
pub mod bind_group_handle;

pub use bind_group_drawing::*;
pub use bind_group_entry_handle::*;
pub use bind_group_entry_ref::*;
pub use bind_group_handle::*;

use crate::{
    frame_graph::PassNodeBuilder,
    render_resource::{Buffer, Texture},
};

use super::TextureViewInfo;

pub trait BindingResourceHelper {
    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef;
}

impl BindingResourceHelper for Buffer {
    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        let buffer = pass_node_builder.read_material(self);

        BindingResourceRef::Buffer { buffer, size: None }
    }
}

impl BindingResourceHelper for Texture {
    fn make_binding_resource_ref(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> BindingResourceRef {
        let texture = pass_node_builder.read_material(self);

        BindingResourceRef::TextureView {
            texture,
            texture_view_info: TextureViewInfo::default(),
        }
    }
}
