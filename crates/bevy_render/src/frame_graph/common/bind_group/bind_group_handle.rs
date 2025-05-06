use std::borrow::Cow;

use crate::{
    frame_graph::{FrameGraphError, PassNodeBuilder, ResourceHandle},
    render_resource::BindGroupLayout,
};

use super::{BindGroupDrawing, BindGroupEntryHandle};

#[derive(Clone)]
pub struct BindGroupHandle {
    pub label: Option<Cow<'static, str>>,
    pub layout: BindGroupLayout,
    pub entries: Vec<BindGroupEntryHandle>,
}

impl ResourceHandle for BindGroupHandle {
    type Drawing = BindGroupDrawing;

    fn make_resource_drawing(
        &self,
        pass_node_builder: &mut PassNodeBuilder,
    ) -> Result<Self::Drawing, FrameGraphError> {
        let entries = self
            .entries
            .iter()
            .map(|entry| entry.get_ref(pass_node_builder))
            .collect::<Vec<_>>();

        Ok(BindGroupDrawing {
            label: self.label.clone(),
            layout: self.layout.clone(),
            entries,
        })
    }
}
