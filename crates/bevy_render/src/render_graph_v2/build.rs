use super::{resource::RenderGraphResourceUsageType, RenderGraph};
use crate::{
    render_resource::{ComputePipelineDescriptor, PipelineCache},
    renderer::RenderDevice,
};
use wgpu::{BindGroupLayoutEntry, BindingType, ShaderStages, StorageTextureAccess, TextureUsages};

impl RenderGraph {
    pub(crate) fn build(&mut self, render_device: &RenderDevice, pipeline_cache: &PipelineCache) {
        self.compute_resource_usages();
        self.build_resources(render_device);
        self.build_bind_group_layouts(render_device);
        self.build_pipelines(pipeline_cache);
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
        for node in &self.nodes {
            for resource_usage in node.resource_usages.iter() {
                if let Some(resource_descriptor) =
                    self.resource_descriptors.get(&resource_usage.resource.id)
                {
                    self.resources
                        .entry(resource_descriptor.clone())
                        .or_insert_with(|| render_device.create_texture(resource_descriptor));
                }
            }
        }
    }

    fn build_bind_group_layouts(&mut self, render_device: &RenderDevice) {
        for node in &mut self.nodes {
            let entries = node
                .resource_usages
                .iter()
                .enumerate()
                .map(|(i, resource_usage)| {
                    let resource_descriptor = self
                        .resource_descriptors
                        .get(&resource_usage.resource.id)
                        .unwrap();

                    let ty = match resource_usage.usage_type {
                        RenderGraphResourceUsageType::ReadTexture => BindingType::Texture {
                            sample_type: todo!(),
                            view_dimension: todo!(),
                            multisampled: todo!(),
                        },
                        RenderGraphResourceUsageType::WriteTexture => BindingType::StorageTexture {
                            access: StorageTextureAccess::WriteOnly,
                            format: todo!(),
                            view_dimension: todo!(),
                        },
                        RenderGraphResourceUsageType::ReadWriteTexture => {
                            BindingType::StorageTexture {
                                access: StorageTextureAccess::ReadWrite,
                                format: todo!(),
                                view_dimension: todo!(),
                            }
                        }
                    };

                    BindGroupLayoutEntry {
                        binding: i as u32,
                        visibility: ShaderStages::COMPUTE,
                        ty,
                        count: None,
                    }
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

    fn build_pipelines(&mut self, pipeline_cache: &PipelineCache) {
        for node in &mut self.nodes {
            let pipeline_descriptor = ComputePipelineDescriptor {
                label: None,
                layout: vec![node.bind_group_layout.clone().unwrap()],
                push_constant_ranges: vec![],
                shader: node.shader.clone(),
                shader_defs: node.shader_defs.clone(),
                entry_point: node.label.into(),
            };

            node.pipeline = Some(
                *self
                    .pipelines
                    .entry(pipeline_descriptor.clone())
                    .or_insert_with(|| pipeline_cache.queue_compute_pipeline(pipeline_descriptor)),
            )
        }
    }

    fn build_bind_groups(&mut self, render_device: &RenderDevice) {
        todo!()
    }
}
