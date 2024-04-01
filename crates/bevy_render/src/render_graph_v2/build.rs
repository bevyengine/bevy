use super::RenderGraph;
use crate::renderer::RenderDevice;
use wgpu::{BindGroupLayoutEntry, ShaderStages};

impl RenderGraph {
    pub(crate) fn build(&mut self, render_device: &RenderDevice) {
        self.build_resources(render_device);
        self.build_bind_group_layouts(render_device);
        self.build_pipelines(render_device);
        self.build_bind_groups(render_device);
    }

    fn build_resources(&self, render_device: &RenderDevice) {
        todo!()
    }

    fn build_bind_group_layouts(&mut self, render_device: &RenderDevice) {
        for node in &mut self.nodes {
            let entries = node
                .resource_usages
                .iter()
                .enumerate()
                .map(|(i, resource_usage)| BindGroupLayoutEntry {
                    binding: i as u32,
                    visibility: ShaderStages::COMPUTE,
                    ty: todo!(),
                    count: None,
                })
                .collect();

            node.bind_group_layout = Some(
                self.bind_group_layouts
                    .entry(entries)
                    .or_insert_with_key(|entries| {
                        render_device.create_bind_group_layout(node.label, entries)
                    })
                    .clone(),
            );
        }
    }

    fn build_pipelines(&self, render_device: &RenderDevice) {
        todo!()
    }

    fn build_bind_groups(&self, render_device: &RenderDevice) {
        todo!()
    }
}
