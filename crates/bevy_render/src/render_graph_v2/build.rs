use super::{
    resource::{RenderGraphResourceUsage, RenderGraphResourceUsageType},
    RenderGraph,
};
use crate::{
    render_resource::{ComputePipelineDescriptor, PipelineCache},
    renderer::RenderDevice,
};
use wgpu::{
    BindGroupEntry, BindGroupLayoutEntry, BindingResource, BindingType, ShaderStages,
    StorageTextureAccess, TextureDescriptor, TextureUsages, TextureViewDescriptor,
    TextureViewDimension,
};

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
                if resource_usage.resource.generation == 0 {
                    if let Some(resource_descriptor) =
                        self.resource_descriptors.get(&resource_usage.resource.id)
                    {
                        self.resources
                            .entry(resource_usage.resource.id)
                            .or_insert_with(|| render_device.create_texture(resource_descriptor));
                    }
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

                    let ty = get_binding_type(resource_usage, resource_descriptor, render_device);

                    BindGroupLayoutEntry {
                        binding: i as u32,
                        visibility: ShaderStages::COMPUTE, // TODO: Don't hardcode
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
                label: None, // TODO: Ideally we can set the bind group to the node label
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
        for node in &mut self.nodes {
            let entries = &node
                .resource_usages
                .iter()
                .enumerate()
                .map(|(i, resource_usage)| {
                    // TODO: Cache view
                    let texture_view = self
                        .resources
                        .get(&resource_usage.resource.id)
                        .unwrap()
                        .create_view(&TextureViewDescriptor {
                            label: todo!(),
                            format: todo!(),
                            dimension: todo!(),
                            aspect: todo!(),
                            base_mip_level: todo!(),
                            mip_level_count: todo!(),
                            base_array_layer: todo!(),
                            array_layer_count: todo!(),
                        });

                    BindGroupEntry {
                        binding: i as u32,
                        resource: BindingResource::TextureView(&texture_view),
                    }
                })
                .collect::<Box<[BindGroupEntry]>>();

            // TODO: Cache bind group
            node.bind_group = Some(render_device.create_bind_group(
                node.label,
                node.bind_group_layout.as_ref().unwrap(),
                entries,
            ));
        }
    }
}

fn get_binding_type(
    resource_usage: &RenderGraphResourceUsage,
    resource_descriptor: &TextureDescriptor,
    render_device: &RenderDevice,
) -> BindingType {
    match resource_usage.usage_type {
        RenderGraphResourceUsageType::ReadTexture => BindingType::Texture {
            sample_type: resource_descriptor
                .format
                .sample_type(None, Some(render_device.features()))
                .unwrap(),
            view_dimension: TextureViewDimension::D2, // TODO: Don't hardcode
            multisampled: resource_descriptor.sample_count != 1,
        },
        RenderGraphResourceUsageType::WriteTexture => BindingType::StorageTexture {
            access: StorageTextureAccess::WriteOnly,
            format: resource_descriptor.format,
            view_dimension: TextureViewDimension::D2, // TODO: Don't hardcode
        },
        RenderGraphResourceUsageType::ReadWriteTexture => {
            BindingType::StorageTexture {
                access: StorageTextureAccess::ReadWrite,
                format: resource_descriptor.format,
                view_dimension: TextureViewDimension::D2, // TODO: Don't hardcode
            }
        }
    }
}
