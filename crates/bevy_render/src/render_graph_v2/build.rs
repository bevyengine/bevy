use super::{resource::RenderGraphResourceUsageType, RenderGraph};
use crate::renderer::RenderDevice;
use wgpu::{BindGroupLayoutEntry, ShaderStages, TextureUsages};

impl RenderGraph {
    pub(crate) fn build(&mut self, render_device: &RenderDevice) {
        self.compute_resource_usages();
        self.build_resources(render_device);
        self.build_bind_group_layouts(render_device);
        self.build_pipelines(render_device);
        self.build_bind_groups(render_device);
    }

    fn compute_resource_usages(&mut self) {
        for node in &self.nodes {
            for resource_usage in node.resource_usages.iter() {
                if let Some(resource_descriptor) = self
                    .resource_descriptors
                    .get_mut(&resource_usage.resource.id)
                {
                    resource_descriptor.usage |= match resource_usage.usage_type {
                        RenderGraphResourceUsageType::ReadTexture => TextureUsages::TEXTURE_BINDING,
                        RenderGraphResourceUsageType::WriteTexture
                        | RenderGraphResourceUsageType::ReadWriteTexture => {
                            TextureUsages::STORAGE_BINDING
                        }
                    };
                }
            }
        }
    }

    fn build_resources(&mut self, render_device: &RenderDevice) {
        // TODO: Create textures if not exist
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

    fn build_pipelines(&mut self, render_device: &RenderDevice) {
        todo!()
    }

    fn build_bind_groups(&mut self, render_device: &RenderDevice) {
        todo!()
    }
}
