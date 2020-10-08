use crate::{
    wgpu_type_converter::{OwnedWgpuVertexBufferDescriptor, WgpuInto},
    WgpuBindGroupInfo, WgpuResources,
};

use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_render::{
    pipeline::{
        BindGroupDescriptor, BindGroupDescriptorId, BindingShaderStage, PipelineDescriptor,
    },
    renderer::{
        BindGroup, BufferId, BufferInfo, RenderResourceBinding, RenderResourceContext,
        RenderResourceId, SamplerId, TextureId,
    },
    shader::Shader,
    texture::{Extent3d, SamplerDescriptor, TextureDescriptor},
};
use bevy_window::{Window, WindowId};
use futures_lite::future;
use std::{borrow::Cow, ops::Range, sync::Arc};
use wgpu::util::DeviceExt;

#[derive(Clone, Debug)]
pub struct WgpuRenderResourceContext {
    pub device: Arc<wgpu::Device>,
    pub resources: WgpuResources,
}

impl WgpuRenderResourceContext {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        WgpuRenderResourceContext {
            device,
            resources: WgpuResources::default(),
        }
    }

    pub fn set_window_surface(&self, window_id: WindowId, surface: wgpu::Surface) {
        let mut window_surfaces = self.resources.window_surfaces.write();
        window_surfaces.insert(window_id, surface);
    }

    pub fn copy_buffer_to_buffer(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        source_buffer: BufferId,
        source_offset: u64,
        destination_buffer: BufferId,
        destination_offset: u64,
        size: u64,
    ) {
        let buffers = self.resources.buffers.read();

        let source = buffers.get(&source_buffer).unwrap();
        let destination = buffers.get(&destination_buffer).unwrap();
        command_encoder.copy_buffer_to_buffer(
            source,
            source_offset,
            destination,
            destination_offset,
            size,
        );
    }

    #[allow(clippy::too_many_arguments)]
    pub fn copy_buffer_to_texture(
        &self,
        command_encoder: &mut wgpu::CommandEncoder,
        source_buffer: BufferId,
        source_offset: u64,
        source_bytes_per_row: u32,
        destination_texture: TextureId,
        destination_origin: [u32; 3], // TODO: replace with math type
        destination_mip_level: u32,
        size: Extent3d,
    ) {
        let buffers = self.resources.buffers.read();
        let textures = self.resources.textures.read();

        let source = buffers.get(&source_buffer).unwrap();
        let destination = textures.get(&destination_texture).unwrap();
        command_encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: source,
                layout: wgpu::TextureDataLayout {
                    offset: source_offset,
                    bytes_per_row: source_bytes_per_row,
                    rows_per_image: 0, // NOTE: Example sets this to 0, but should it be height?
                },
            },
            wgpu::TextureCopyView {
                texture: destination,
                mip_level: destination_mip_level,
                origin: wgpu::Origin3d {
                    x: destination_origin[0],
                    y: destination_origin[1],
                    z: destination_origin[2],
                },
            },
            size.wgpu_into(),
        );
    }

    pub fn create_bind_group_layout(&self, descriptor: &BindGroupDescriptor) {
        if self
            .resources
            .bind_group_layouts
            .read()
            .get(&descriptor.id)
            .is_some()
        {
            return;
        }

        let mut bind_group_layouts = self.resources.bind_group_layouts.write();
        // TODO: consider re-checking existence here
        let bind_group_layout_entries = descriptor
            .bindings
            .iter()
            .map(|binding| {
                let shader_stage = if binding.shader_stage
                    == BindingShaderStage::VERTEX | BindingShaderStage::FRAGMENT
                {
                    wgpu::ShaderStage::VERTEX | wgpu::ShaderStage::FRAGMENT
                } else if binding.shader_stage == BindingShaderStage::VERTEX {
                    wgpu::ShaderStage::VERTEX
                } else if binding.shader_stage == BindingShaderStage::FRAGMENT {
                    wgpu::ShaderStage::FRAGMENT
                } else {
                    panic!("Invalid binding shader stage.")
                };
                wgpu::BindGroupLayoutEntry {
                    binding: binding.index,
                    visibility: shader_stage,
                    ty: (&binding.bind_type).wgpu_into(),
                    count: None,
                }
            })
            .collect::<Vec<wgpu::BindGroupLayoutEntry>>();
        let wgpu_descriptor = wgpu::BindGroupLayoutDescriptor {
            entries: bind_group_layout_entries.as_slice(),
            label: None,
        };
        let bind_group_layout = self.device.create_bind_group_layout(&wgpu_descriptor);
        bind_group_layouts.insert(descriptor.id, bind_group_layout);
    }

    fn try_next_swap_chain_texture(&self, window_id: bevy_window::WindowId) -> Option<TextureId> {
        let mut window_swap_chains = self.resources.window_swap_chains.write();
        let mut swap_chain_outputs = self.resources.swap_chain_frames.write();

        let window_swap_chain = window_swap_chains.get_mut(&window_id).unwrap();
        let next_texture = window_swap_chain.get_current_frame().ok()?;
        let id = TextureId::new();
        swap_chain_outputs.insert(id, next_texture);
        Some(id)
    }
}

impl RenderResourceContext for WgpuRenderResourceContext {
    fn create_sampler(&self, sampler_descriptor: &SamplerDescriptor) -> SamplerId {
        let mut samplers = self.resources.samplers.write();

        let descriptor: wgpu::SamplerDescriptor = (*sampler_descriptor).wgpu_into();
        let sampler = self.device.create_sampler(&descriptor);

        let id = SamplerId::new();
        samplers.insert(id, sampler);
        id
    }

    fn create_texture(&self, texture_descriptor: TextureDescriptor) -> TextureId {
        let mut textures = self.resources.textures.write();
        let mut texture_views = self.resources.texture_views.write();
        let mut texture_descriptors = self.resources.texture_descriptors.write();

        let descriptor: wgpu::TextureDescriptor = (&texture_descriptor).wgpu_into();
        let texture = self.device.create_texture(&descriptor);
        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let id = TextureId::new();
        texture_descriptors.insert(id, texture_descriptor);
        texture_views.insert(id, texture_view);
        textures.insert(id, texture);
        id
    }

    fn create_buffer(&self, buffer_info: BufferInfo) -> BufferId {
        // TODO: consider moving this below "create" for efficiency
        let mut buffer_infos = self.resources.buffer_infos.write();
        let mut buffers = self.resources.buffers.write();

        let buffer = self.device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_info.size as u64,
            usage: buffer_info.buffer_usage.wgpu_into(),
            mapped_at_creation: buffer_info.mapped_at_creation,
        });

        let id = BufferId::new();
        buffer_infos.insert(id, buffer_info);
        buffers.insert(id, Arc::new(buffer));
        id
    }

    fn create_buffer_with_data(&self, mut buffer_info: BufferInfo, data: &[u8]) -> BufferId {
        // TODO: consider moving this below "create" for efficiency
        let mut buffer_infos = self.resources.buffer_infos.write();
        let mut buffers = self.resources.buffers.write();

        buffer_info.size = data.len();
        let buffer = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                contents: data,
                label: None,
                usage: buffer_info.buffer_usage.wgpu_into(),
            });

        let id = BufferId::new();
        buffer_infos.insert(id, buffer_info);
        buffers.insert(id, Arc::new(buffer));
        id
    }

    fn remove_buffer(&self, buffer: BufferId) {
        let mut buffers = self.resources.buffers.write();
        let mut buffer_infos = self.resources.buffer_infos.write();

        buffers.remove(&buffer);
        buffer_infos.remove(&buffer);
    }

    fn remove_texture(&self, texture: TextureId) {
        let mut textures = self.resources.textures.write();
        let mut texture_views = self.resources.texture_views.write();
        let mut texture_descriptors = self.resources.texture_descriptors.write();

        textures.remove(&texture);
        texture_views.remove(&texture);
        texture_descriptors.remove(&texture);
    }

    fn remove_sampler(&self, sampler: SamplerId) {
        let mut samplers = self.resources.samplers.write();
        samplers.remove(&sampler);
    }

    fn create_shader_module_from_source(&self, shader_handle: Handle<Shader>, shader: &Shader) {
        let mut shader_modules = self.resources.shader_modules.write();
        let spirv: Cow<[u32]> = shader.get_spirv(None).into();
        let shader_module = self
            .device
            .create_shader_module(wgpu::ShaderModuleSource::SpirV(spirv));
        shader_modules.insert(shader_handle, shader_module);
    }

    fn create_shader_module(&self, shader_handle: Handle<Shader>, shaders: &Assets<Shader>) {
        if self
            .resources
            .shader_modules
            .read()
            .get(&shader_handle)
            .is_some()
        {
            return;
        }
        let shader = shaders.get(&shader_handle).unwrap();
        self.create_shader_module_from_source(shader_handle, shader);
    }

    fn create_swap_chain(&self, window: &Window) {
        let surfaces = self.resources.window_surfaces.read();
        let mut window_swap_chains = self.resources.window_swap_chains.write();

        let swap_chain_descriptor: wgpu::SwapChainDescriptor = window.wgpu_into();
        let surface = surfaces
            .get(&window.id)
            .expect("No surface found for window");
        let swap_chain = self
            .device
            .create_swap_chain(surface, &swap_chain_descriptor);

        window_swap_chains.insert(window.id, swap_chain);
    }

    fn next_swap_chain_texture(&self, window: &bevy_window::Window) -> TextureId {
        if let Some(texture_id) = self.try_next_swap_chain_texture(window.id) {
            texture_id
        } else {
            self.resources.window_swap_chains.write().remove(&window.id);
            self.create_swap_chain(window);
            self.try_next_swap_chain_texture(window.id)
                .expect("Failed to acquire next swap chain texture!")
        }
    }

    fn drop_swap_chain_texture(&self, texture: TextureId) {
        let mut swap_chain_outputs = self.resources.swap_chain_frames.write();
        swap_chain_outputs.remove(&texture);
    }

    fn drop_all_swap_chain_textures(&self) {
        let mut swap_chain_outputs = self.resources.swap_chain_frames.write();
        swap_chain_outputs.clear();
    }

    fn set_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        render_resource: RenderResourceId,
        index: usize,
    ) {
        let mut asset_resources = self.resources.asset_resources.write();
        asset_resources.insert((handle, index), render_resource);
    }

    fn get_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        index: usize,
    ) -> Option<RenderResourceId> {
        let asset_resources = self.resources.asset_resources.read();
        asset_resources.get(&(handle, index)).cloned()
    }

    fn remove_asset_resource_untyped(&self, handle: HandleUntyped, index: usize) {
        let mut asset_resources = self.resources.asset_resources.write();
        asset_resources.remove(&(handle, index));
    }

    fn create_render_pipeline(
        &self,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline_descriptor: &PipelineDescriptor,
        shaders: &Assets<Shader>,
    ) {
        if self
            .resources
            .render_pipelines
            .read()
            .get(&pipeline_handle)
            .is_some()
        {
            return;
        }

        let layout = pipeline_descriptor.get_layout().unwrap();
        for bind_group_descriptor in layout.bind_groups.iter() {
            self.create_bind_group_layout(&bind_group_descriptor);
        }

        let bind_group_layouts = self.resources.bind_group_layouts.read();
        // setup and collect bind group layouts
        let bind_group_layouts = layout
            .bind_groups
            .iter()
            .map(|bind_group| bind_group_layouts.get(&bind_group.id).unwrap())
            .collect::<Vec<&wgpu::BindGroupLayout>>();

        let pipeline_layout = self
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: None,
                bind_group_layouts: bind_group_layouts.as_slice(),
                push_constant_ranges: &[],
            });

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

        self.create_shader_module(pipeline_descriptor.shader_stages.vertex, shaders);

        if let Some(fragment_handle) = pipeline_descriptor.shader_stages.fragment {
            self.create_shader_module(fragment_handle, shaders);
        }

        let shader_modules = self.resources.shader_modules.read();
        let vertex_shader_module = shader_modules
            .get(&pipeline_descriptor.shader_stages.vertex)
            .unwrap();

        let fragment_shader_module = match pipeline_descriptor.shader_stages.fragment {
            Some(fragment_handle) => Some(shader_modules.get(&fragment_handle).unwrap()),
            None => None,
        };

        let render_pipeline_descriptor = wgpu::RenderPipelineDescriptor {
            label: None,
            layout: Some(&pipeline_layout),
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

        let render_pipeline = self
            .device
            .create_render_pipeline(&render_pipeline_descriptor);
        let mut render_pipelines = self.resources.render_pipelines.write();
        render_pipelines.insert(pipeline_handle, render_pipeline);
    }

    fn bind_group_descriptor_exists(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
    ) -> bool {
        let bind_group_layouts = self.resources.bind_group_layouts.read();
        bind_group_layouts.get(&bind_group_descriptor_id).is_some()
    }

    fn create_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        bind_group: &BindGroup,
    ) {
        if !self
            .resources
            .has_bind_group(bind_group_descriptor_id, bind_group.id)
        {
            log::trace!(
                "start creating bind group for RenderResourceSet {:?}",
                bind_group.id
            );
            let texture_views = self.resources.texture_views.read();
            let samplers = self.resources.samplers.read();
            let buffers = self.resources.buffers.read();
            let bind_group_layouts = self.resources.bind_group_layouts.read();
            let mut bind_groups = self.resources.bind_groups.write();

            let entries = bind_group
                .indexed_bindings
                .iter()
                .map(|indexed_binding| {
                    let wgpu_resource = match &indexed_binding.entry {
                        RenderResourceBinding::Texture(resource) => {
                            let texture_view = texture_views
                                .get(&resource)
                                .unwrap_or_else(|| panic!("{:?}", resource));
                            wgpu::BindingResource::TextureView(texture_view)
                        }
                        RenderResourceBinding::Sampler(resource) => {
                            let sampler = samplers.get(&resource).unwrap();
                            wgpu::BindingResource::Sampler(sampler)
                        }
                        RenderResourceBinding::Buffer { buffer, range, .. } => {
                            let wgpu_buffer = buffers.get(&buffer).unwrap();
                            wgpu::BindingResource::Buffer(wgpu_buffer.slice(range.clone()))
                        }
                    };
                    wgpu::BindGroupEntry {
                        binding: indexed_binding.index,
                        resource: wgpu_resource,
                    }
                })
                .collect::<Vec<wgpu::BindGroupEntry>>();

            let bind_group_layout = bind_group_layouts.get(&bind_group_descriptor_id).unwrap();
            let wgpu_bind_group_descriptor = wgpu::BindGroupDescriptor {
                label: None,
                layout: bind_group_layout,
                entries: entries.as_slice(),
            };
            let wgpu_bind_group = self.device.create_bind_group(&wgpu_bind_group_descriptor);

            let bind_group_info = bind_groups
                .entry(bind_group_descriptor_id)
                .or_insert_with(WgpuBindGroupInfo::default);
            bind_group_info
                .bind_groups
                .insert(bind_group.id, wgpu_bind_group);
            log::trace!(
                "created bind group for RenderResourceSet {:?}",
                bind_group.id
            );
        }
    }

    fn clear_bind_groups(&self) {
        self.resources.bind_groups.write().clear();
    }

    fn get_buffer_info(&self, buffer: BufferId) -> Option<BufferInfo> {
        self.resources.buffer_infos.read().get(&buffer).cloned()
    }

    fn write_mapped_buffer(
        &self,
        id: BufferId,
        range: Range<u64>,
        write: &mut dyn FnMut(&mut [u8], &dyn RenderResourceContext),
    ) {
        let buffer = {
            let buffers = self.resources.buffers.read();
            buffers.get(&id).unwrap().clone()
        };
        let buffer_slice = buffer.slice(range);
        let mut data = buffer_slice.get_mapped_range_mut();
        write(&mut data, self);
    }

    fn map_buffer(&self, id: BufferId) {
        let buffers = self.resources.buffers.read();
        let buffer = buffers.get(&id).unwrap();
        let buffer_slice = buffer.slice(..);
        let data = buffer_slice.map_async(wgpu::MapMode::Write);
        self.device.poll(wgpu::Maintain::Wait);
        if future::block_on(data).is_err() {
            panic!("failed to map buffer to host");
        }
    }

    fn unmap_buffer(&self, id: BufferId) {
        let buffers = self.resources.buffers.read();
        let buffer = buffers.get(&id).unwrap();
        buffer.unmap();
    }
}
