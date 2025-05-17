use std::borrow::Cow;

use crate::{
    frame_graph::{FrameGraph, PassNodeBuilder},
    render_resource::BindGroupLayout,
};

use super::{
    BindGroupBinding, BindGroupEntryHandle, BindGroupResourceHandleHelper,
    IntoBindGroupResourceHandle,
};

pub struct BindGroupHandleBuilder<'a> {
    pub label: Option<Cow<'static, str>>,
    pub layout: BindGroupLayout,
    pub entries: Vec<BindGroupEntryHandle>,
    frame_graph: &'a mut FrameGraph,
}

impl<'a> BindGroupHandleBuilder<'a> {
    pub fn new(
        label: Option<Cow<'static, str>>,
        layout: BindGroupLayout,
        frame_graph: &'a mut FrameGraph,
    ) -> Self {
        Self {
            label,
            layout,
            entries: vec![],
            frame_graph,
        }
    }

    pub fn add_handle<T: IntoBindGroupResourceHandle>(
        mut self,
        binding: u32,
        handle: T,
    ) -> Self {
        self.entries.push(BindGroupEntryHandle {
            binding,
            resource: handle.into_binding(),
        });

        self
    }

    pub fn add_helper<T: BindGroupResourceHandleHelper>(self, binding: u32, value: &T) -> Self {
        let handle = value.make_bind_group_resource_handle(self.frame_graph);
        self.add_handle(binding, handle)
    }

    pub fn build(self) -> BindGroupHandle {
        BindGroupHandle {
            label: self.label,
            layout: self.layout,
            entries: self.entries,
        }
    }
}

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
