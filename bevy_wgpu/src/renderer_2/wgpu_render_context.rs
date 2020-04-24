use super::WgpuRenderResourceContext;
use crate::{
    wgpu_type_converter::{OwnedWgpuVertexBufferDescriptor, WgpuInto},
    WgpuRenderPass, WgpuResourceRefs,
};
use bevy_asset::{AssetStorage, Handle};
use bevy_render::{
    pass::{
        PassDescriptor, RenderPass, RenderPassColorAttachmentDescriptor,
        RenderPassDepthStencilAttachmentDescriptor, TextureAttachment,
    },
    pipeline::{BindGroupDescriptor, BindType, PipelineDescriptor},
    render_resource::{
        RenderResource, RenderResourceAssignments, RenderResourceSetId, ResourceInfo,
    },
    renderer_2::{RenderContext, RenderResourceContext},
    shader::Shader,
    texture::{Extent3d, TextureDescriptor},
};
use bevy_window::WindowId;
use std::{collections::HashMap, sync::Arc};

#[derive(Default)]
pub struct LazyCommandEncoder {
    command_encoder: Option<wgpu::CommandEncoder>,
}

impl LazyCommandEncoder {
    pub fn get_or_create(&mut self, device: &wgpu::Device) -> &mut wgpu::CommandEncoder {
        match self.command_encoder {
            Some(ref mut command_encoder) => command_encoder,
            None => {
                self.create(device);
                self.command_encoder.as_mut().unwrap()
            }
        }
    }

    pub fn is_some(&self) -> bool {
        self.command_encoder.is_some()
    }

    pub fn create(&mut self, device: &wgpu::Device) {
        let command_encoder =
            device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
        self.command_encoder = Some(command_encoder);
    }

    pub fn take(&mut self) -> Option<wgpu::CommandEncoder> {
        self.command_encoder.take()
    }

    pub fn set(&mut self, command_encoder: wgpu::CommandEncoder) {
        self.command_encoder = Some(command_encoder);
    }
}

pub struct WgpuRenderContext {
    pub device: Arc<wgpu::Device>,
    // TODO: remove this
    pub primary_window: Option<WindowId>,
    pub command_encoder: LazyCommandEncoder,
    pub render_resources: WgpuRenderResourceContext,
}

impl WgpuRenderContext {
    pub fn new(device: Arc<wgpu::Device>, resources: WgpuRenderResourceContext) -> Self {
        WgpuRenderContext {
            device,
            primary_window: None,
            render_resources: resources,
            command_encoder: LazyCommandEncoder::default(),
        }
    }

    /// Consume this context, finalize the current CommandEncoder (if it exists), and take the current WgpuResources.
    /// This is intended to be called from a worker thread right before synchronizing with the main thread.   
    pub fn finish(&mut self) -> Option<wgpu::CommandBuffer> {
        self.command_encoder.take().map(|encoder| encoder.finish())
    }
}

impl RenderContext for WgpuRenderContext {
    fn create_texture_with_data(
        &mut self,
        texture_descriptor: &TextureDescriptor,
        bytes: &[u8],
    ) -> RenderResource {
        self.render_resources
            .wgpu_resources
            .create_texture_with_data(
                &self.device,
                self.command_encoder.get_or_create(&self.device),
                texture_descriptor,
                bytes,
            )
    }
    fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    ) {
        let command_encoder = self.command_encoder.get_or_create(&self.device);
        self.render_resources.wgpu_resources.copy_buffer_to_buffer(
            command_encoder,
            source_buffer,
            source_offset,
            destination_buffer,
            destination_offset,
            size,
        );
    }
    fn resources(&self) -> &dyn RenderResourceContext {
        &self.render_resources
    }
    fn resources_mut(&mut self) -> &mut dyn RenderResourceContext {
        &mut self.render_resources
    }
    fn create_bind_group(
        &mut self,
        bind_group_descriptor: &BindGroupDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> Option<RenderResourceSetId> {
        if let Some((render_resource_set_id, _indices)) =
            render_resource_assignments.get_render_resource_set_id(bind_group_descriptor.id)
        {
            if !self
                .render_resources
                .wgpu_resources
                .has_bind_group(bind_group_descriptor.id, *render_resource_set_id)
            {
                log::trace!(
                    "start creating bind group for RenderResourceSet {:?}",
                    render_resource_set_id
                );
                let wgpu_bind_group = {
                    let textures = self
                        .render_resources
                        .wgpu_resources
                        .texture_views
                        .read()
                        .unwrap();
                    let samplers = self
                        .render_resources
                        .wgpu_resources
                        .samplers
                        .read()
                        .unwrap();
                    let buffers = self.render_resources.wgpu_resources.buffers.read().unwrap();
                    let bindings = bind_group_descriptor
                        .bindings
                        .iter()
                        .map(|binding| {
                            if let Some(resource) = render_resource_assignments.get(&binding.name) {
                                let mut wgpu_resource = None;
                                self.resources().get_resource_info(
                                    resource,
                                    &mut |resource_info| {
                                        log::trace!(
                                            "found binding {} ({}) resource: {:?} {:?}",
                                            binding.index,
                                            binding.name,
                                            resource,
                                            resource_info
                                        );
                                        wgpu_resource = match &binding.bind_type {
                                            BindType::SampledTexture { .. } => {
                                                if let Some(ResourceInfo::Texture) = resource_info {
                                                    let texture = textures.get(&resource).unwrap();
                                                    Some(wgpu::BindingResource::TextureView(
                                                        texture,
                                                    ))
                                                } else {
                                                    panic!("expected a Texture resource");
                                                }
                                            }
                                            BindType::Sampler { .. } => {
                                                if let Some(ResourceInfo::Sampler) = resource_info {
                                                    let sampler = samplers.get(&resource).unwrap();
                                                    Some(wgpu::BindingResource::Sampler(sampler))
                                                } else {
                                                    panic!("expected a Sampler resource");
                                                }
                                            }
                                            BindType::Uniform { .. } => {
                                                if let Some(ResourceInfo::Buffer(buffer_info)) =
                                                    resource_info
                                                {
                                                    let buffer = buffers.get(&resource).unwrap();
                                                    Some(wgpu::BindingResource::Buffer {
                                                        buffer,
                                                        range: 0..buffer_info.size as u64,
                                                    })
                                                } else {
                                                    panic!("expected a Buffer resource");
                                                }
                                            }
                                            _ => panic!("unsupported bind type"),
                                        }
                                    },
                                );
                                wgpu::Binding {
                                    binding: binding.index,
                                    resource: wgpu_resource.expect("No resource binding found"),
                                }
                            } else {
                                panic!(
                        "No resource assigned to uniform \"{}\" for RenderResourceAssignments {:?}",
                        binding.name,
                        render_resource_assignments.id
                    );
                            }
                        })
                        .collect::<Vec<wgpu::Binding>>();
                    let bind_group_layouts = self
                        .render_resources
                        .wgpu_resources
                        .bind_group_layouts
                        .read()
                        .unwrap();
                    let bind_group_layout =
                        bind_group_layouts.get(&bind_group_descriptor.id).unwrap();
                    let wgpu_bind_group_descriptor = wgpu::BindGroupDescriptor {
                        label: None,
                        layout: bind_group_layout,
                        bindings: bindings.as_slice(),
                    };
                    self.render_resources.wgpu_resources.create_bind_group(
                        &self.device,
                        *render_resource_set_id,
                        &wgpu_bind_group_descriptor,
                    )
                };
                self.render_resources.wgpu_resources.set_bind_group(
                    bind_group_descriptor.id,
                    *render_resource_set_id,
                    wgpu_bind_group,
                );
                return Some(*render_resource_set_id);
            }
        }

        None
    }
    fn create_render_pipeline(
        &mut self,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &PipelineDescriptor,
        shader_storage: &AssetStorage<Shader>,
    ) {
        if let Some(_) = self
            .render_resources
            .wgpu_resources
            .render_pipelines
            .read()
            .unwrap()
            .get(&pipeline_handle)
        {
            return;
        }

        let layout = pipeline_descriptor.get_layout().unwrap();
        for bind_group in layout.bind_groups.iter() {
            if self
                .render_resources
                .wgpu_resources
                .bind_group_layouts
                .read()
                .unwrap()
                .get(&bind_group.id)
                .is_none()
            {
                let bind_group_layout_binding = bind_group
                    .bindings
                    .iter()
                    .map(|binding| wgpu::BindGroupLayoutEntry {
                        binding: binding.index,
                        visibility: wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT,
                        ty: (&binding.bind_type).wgpu_into(),
                    })
                    .collect::<Vec<wgpu::BindGroupLayoutEntry>>();
                self.render_resources
                    .wgpu_resources
                    .create_bind_group_layout(
                        &self.device,
                        bind_group.id,
                        &wgpu::BindGroupLayoutDescriptor {
                            bindings: bind_group_layout_binding.as_slice(),
                            label: None,
                        },
                    );
            }
        }

        let pipeline_layout = {
            let bind_group_layouts = self
                .render_resources
                .wgpu_resources
                .bind_group_layouts
                .read()
                .unwrap();
            // setup and collect bind group layouts
            let bind_group_layouts = layout
                .bind_groups
                .iter()
                .map(|bind_group| bind_group_layouts.get(&bind_group.id).unwrap())
                .collect::<Vec<&wgpu::BindGroupLayout>>();
            self.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    bind_group_layouts: bind_group_layouts.as_slice(),
                })
        };

        let owned_vertex_buffer_descriptors = layout
            .vertex_buffer_descriptors
            .iter()
            .map(|v| v.wgpu_into())
            .collect::<Vec<OwnedWgpuVertexBufferDescriptor>>();

        let color_states = pipeline_descriptor
            .color_states
            .iter()
            .map(|c| c.wgpu_into())
            .collect::<Vec<wgpu::ColorStateDescriptor>>();

        if self
            .render_resources
            .wgpu_resources
            .shader_modules
            .read()
            .unwrap()
            .get(&pipeline_descriptor.shader_stages.vertex)
            .is_none()
        {
            self.render_resources
                .create_shader_module(pipeline_descriptor.shader_stages.vertex, shader_storage);
        }

        if let Some(fragment_handle) = pipeline_descriptor.shader_stages.fragment {
            if self
                .render_resources
                .wgpu_resources
                .shader_modules
                .read()
                .unwrap()
                .get(&fragment_handle)
                .is_none()
            {
                self.render_resources
                    .create_shader_module(fragment_handle, shader_storage);
            }
        };
        let wgpu_pipeline = {
            let shader_modules = self
                .render_resources
                .wgpu_resources
                .shader_modules
                .read()
                .unwrap();
            let vertex_shader_module = shader_modules
                .get(&pipeline_descriptor.shader_stages.vertex)
                .unwrap();

            let fragment_shader_module = match pipeline_descriptor.shader_stages.fragment {
                Some(fragment_handle) => Some(shader_modules.get(&fragment_handle).unwrap()),
                None => None,
            };

            let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
                layout: &pipeline_layout,
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vertex_shader_module,
                    entry_point: "main",
                },
                fragment_stage: match pipeline_descriptor.shader_stages.fragment {
                    Some(_) => Some(wgpu::ProgrammableStageDescriptor {
                        entry_point: "main",
                        module: fragment_shader_module.as_ref().unwrap(),
                    }),
                    None => None,
                },
                rasterization_state: pipeline_descriptor
                    .rasterization_state
                    .as_ref()
                    .map(|r| r.wgpu_into()),
                primitive_topology: pipeline_descriptor.primitive_topology.wgpu_into(),
                color_states: &color_states,
                depth_stencil_state: pipeline_descriptor
                    .depth_stencil_state
                    .as_ref()
                    .map(|d| d.wgpu_into()),
                vertex_state: wgpu::VertexStateDescriptor {
                    index_format: pipeline_descriptor.index_format.wgpu_into(),
                    vertex_buffers: &owned_vertex_buffer_descriptors
                        .iter()
                        .map(|v| v.into())
                        .collect::<Vec<wgpu::VertexBufferDescriptor>>(),
                },
                sample_count: pipeline_descriptor.sample_count,
                sample_mask: pipeline_descriptor.sample_mask,
                alpha_to_coverage_enabled: pipeline_descriptor.alpha_to_coverage_enabled,
            };

            self.render_resources
                .wgpu_resources
                .create_render_pipeline(&self.device, &render_pipeline_descriptor)
        };
        self.render_resources
            .wgpu_resources
            .set_render_pipeline(pipeline_handle, wgpu_pipeline);
    }
    fn begin_pass(
        &mut self,
        pass_descriptor: &PassDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
        run_pass: &mut dyn Fn(&mut dyn RenderPass),
    ) {
        if !self.command_encoder.is_some() {
            self.command_encoder.create(&self.device);
        }
        let resource_lock = self.render_resources.wgpu_resources.read();
        let refs = resource_lock.refs();
        let mut encoder = self.command_encoder.take().unwrap();
        {
            let render_pass = create_render_pass(
                pass_descriptor,
                render_resource_assignments,
                &refs,
                &mut encoder,
            );
            let mut wgpu_render_pass = WgpuRenderPass {
                render_context: self,
                render_pass,
                render_resources: refs,
                bound_bind_groups: HashMap::default(),
            };

            run_pass(&mut wgpu_render_pass);
        }

        self.command_encoder.set(encoder);
    }
    fn copy_buffer_to_texture(
        &mut self,
        source_buffer: RenderResource,
        source_offset: u64,
        source_bytes_per_row: u32,
        destination_texture: RenderResource,
        destination_origin: [u32; 3],
        destination_mip_level: u32,
        destination_array_layer: u32,
        size: Extent3d,
    ) {
        self.render_resources.wgpu_resources.copy_buffer_to_texture(
            self.command_encoder.get_or_create(&self.device),
            source_buffer,
            source_offset,
            source_bytes_per_row,
            destination_texture,
            destination_origin,
            destination_mip_level,
            destination_array_layer,
            size,
        )
    }
}

pub fn create_render_pass<'a, 'b>(
    pass_descriptor: &PassDescriptor,
    global_render_resource_assignments: &'b RenderResourceAssignments,
    refs: &WgpuResourceRefs<'a>,
    encoder: &'a mut wgpu::CommandEncoder,
) -> wgpu::RenderPass<'a> {
    encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        color_attachments: &pass_descriptor
            .color_attachments
            .iter()
            .map(|c| {
                create_wgpu_color_attachment_descriptor(global_render_resource_assignments, refs, c)
            })
            .collect::<Vec<wgpu::RenderPassColorAttachmentDescriptor>>(),
        depth_stencil_attachment: pass_descriptor.depth_stencil_attachment.as_ref().map(|d| {
            create_wgpu_depth_stencil_attachment_descriptor(
                global_render_resource_assignments,
                refs,
                d,
            )
        }),
    })
}

fn get_texture_view<'a>(
    global_render_resource_assignments: &RenderResourceAssignments,
    refs: &WgpuResourceRefs<'a>,
    attachment: &TextureAttachment,
) -> &'a wgpu::TextureView {
    match attachment {
        TextureAttachment::Name(name) => match global_render_resource_assignments.get(&name) {
            Some(resource) => refs.textures.get(&resource).unwrap(),
            None => {
                panic!("Color attachment {} does not exist", name);
            }
        },
        TextureAttachment::RenderResource(render_resource) => refs.textures.get(&render_resource).unwrap_or_else(|| &refs.swap_chain_outputs.get(&render_resource).unwrap().view),
        TextureAttachment::Input(_) => panic!("Encountered unset TextureAttachment::Input. The RenderGraph executor should always set TextureAttachment::Inputs to TextureAttachment::RenderResource before running. This is a bug"),
    }
}

fn create_wgpu_color_attachment_descriptor<'a>(
    global_render_resource_assignments: &RenderResourceAssignments,
    refs: &WgpuResourceRefs<'a>,
    color_attachment_descriptor: &RenderPassColorAttachmentDescriptor,
) -> wgpu::RenderPassColorAttachmentDescriptor<'a> {
    let attachment = get_texture_view(
        global_render_resource_assignments,
        refs,
        &color_attachment_descriptor.attachment,
    );

    let resolve_target = color_attachment_descriptor
        .resolve_target
        .as_ref()
        .map(|target| get_texture_view(global_render_resource_assignments, refs, &target));

    wgpu::RenderPassColorAttachmentDescriptor {
        store_op: color_attachment_descriptor.store_op.wgpu_into(),
        load_op: color_attachment_descriptor.load_op.wgpu_into(),
        clear_color: color_attachment_descriptor.clear_color.wgpu_into(),
        attachment,
        resolve_target,
    }
}

fn create_wgpu_depth_stencil_attachment_descriptor<'a>(
    global_render_resource_assignments: &RenderResourceAssignments,
    refs: &WgpuResourceRefs<'a>,
    depth_stencil_attachment_descriptor: &RenderPassDepthStencilAttachmentDescriptor,
) -> wgpu::RenderPassDepthStencilAttachmentDescriptor<'a> {
    let attachment = get_texture_view(
        global_render_resource_assignments,
        refs,
        &depth_stencil_attachment_descriptor.attachment,
    );

    wgpu::RenderPassDepthStencilAttachmentDescriptor {
        attachment,
        clear_depth: depth_stencil_attachment_descriptor.clear_depth,
        clear_stencil: depth_stencil_attachment_descriptor.clear_stencil,
        depth_load_op: depth_stencil_attachment_descriptor
            .depth_load_op
            .wgpu_into(),
        depth_store_op: depth_stencil_attachment_descriptor
            .depth_store_op
            .wgpu_into(),
        stencil_load_op: depth_stencil_attachment_descriptor
            .stencil_load_op
            .wgpu_into(),
        stencil_store_op: depth_stencil_attachment_descriptor
            .stencil_store_op
            .wgpu_into(),
    }
}
