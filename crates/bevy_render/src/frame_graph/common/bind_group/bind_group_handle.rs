use std::borrow::Cow;

use crate::{frame_graph::PassNodeBuilder, render_resource::BindGroupLayout};

use super::{BindGroupBinding, BindGroupEntryHandle};

#[derive(Clone)]
pub struct BindGroupHandle {
    pub label: Option<Cow<'static, str>>,
    pub layout: BindGroupLayout,
    pub entries: Vec<BindGroupEntryHandle>,
}

impl BindGroupHandle {
    pub fn make_binding(&self, pass_node_builder: &mut PassNodeBuilder) -> BindGroupBinding {
        let entries = self
            .entries
            .iter()
            .map(|entry| entry.get_ref(pass_node_builder))
            .collect::<Vec<_>>();

        BindGroupBinding {
            label: self.label.clone(),
            layout: self.layout.clone(),
            entries,
        }
    }
}
