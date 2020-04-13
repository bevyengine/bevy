use crate::{
    renderer_2::{WgpuRenderResourceContext, WgpuTransactionalRenderResourceContext},
    wgpu_type_converter::WgpuInto,
};
use bevy_asset::Handle;
use bevy_render::{
    pipeline::{BindGroupDescriptorId, PipelineDescriptor},
    render_resource::{
        AssetResources, BufferInfo, RenderResource, RenderResourceSetId, ResourceInfo,
    },
    renderer_2::RenderResourceContext,
    shader::Shader,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_window::{Window, WindowId};
use std::collections::HashMap;

#[derive(Default)]
pub struct WgpuBindGroupInfo {
    pub bind_groups: HashMap<RenderResourceSetId, wgpu::BindGroup>,
}

#[derive(Default)]
pub struct WgpuResources {
    // TODO: remove this from WgpuResources. it doesn't need to be here
    pub asset_resources: AssetResources,
    pub window_surfaces: HashMap<WindowId, wgpu::Surface>,
    pub window_swap_chains: HashMap<WindowId, wgpu::SwapChain>,
    pub swap_chain_outputs: HashMap<WindowId, wgpu::SwapChainOutput>,
    pub buffers: HashMap<RenderResource, wgpu::Buffer>,
    pub textures: HashMap<RenderResource, wgpu::TextureView>,
    pub samplers: HashMap<RenderResource, wgpu::Sampler>,
    pub resource_info: HashMap<RenderResource, ResourceInfo>,
    pub shader_modules: HashMap<Handle<Shader>, wgpu::ShaderModule>,
    pub render_pipelines: HashMap<Handle<PipelineDescriptor>, wgpu::RenderPipeline>,
    pub bind_groups: HashMap<BindGroupDescriptorId, WgpuBindGroupInfo>,
    pub bind_group_layouts: HashMap<BindGroupDescriptorId, wgpu::BindGroupLayout>,
}

impl WgpuResources {
    pub fn consume(&mut self, wgpu_resources: WgpuResources) {
        // TODO: this is brittle. consider a single change-stream-based approach instead?
        self.asset_resources.consume(wgpu_resources.asset_resources);
        self.window_surfaces.extend(wgpu_resources.window_surfaces);
        self.window_swap_chains
            .extend(wgpu_resources.window_swap_chains);
        self.swap_chain_outputs
            .extend(wgpu_resources.swap_chain_outputs);
        self.buffers.extend(wgpu_resources.buffers);
        self.textures.extend(wgpu_resources.textures);
        self.samplers.extend(wgpu_resources.samplers);
        self.resource_info.extend(wgpu_resources.resource_info);
        self.shader_modules.extend(wgpu_resources.shader_modules);
        self.render_pipelines.extend(wgpu_resources.render_pipelines);
        self.bind_groups.extend(wgpu_resources.bind_groups);
        self.bind_group_layouts
            .extend(wgpu_resources.bind_group_layouts);
    }

    pub fn set_window_surface(&mut self, window_id: WindowId, surface: wgpu::Surface) {
        self.window_surfaces.insert(window_id, surface);
    }

    pub fn get_window_surface(&mut self, window_id: WindowId) -> Option<&wgpu::Surface> {
        self.window_surfaces.get(&window_id)
    }

    pub fn next_swap_chain_texture(&mut self, window_id: WindowId) -> Option<&wgpu::SwapChainOutput> {
        let swap_chain_output = self.window_swap_chains.get_mut(&window_id).unwrap();
        let next_texture = swap_chain_output.get_next_texture().unwrap();
        self.swap_chain_outputs.insert(window_id, next_texture);
        self.swap_chain_outputs.get(&window_id)
    }

    pub fn remove_swap_chain_texture(&mut self, window_id: WindowId) {
        self.swap_chain_outputs.remove(&window_id);
    }

    pub fn remove_all_swap_chain_textures(&mut self) {
        self.swap_chain_outputs.clear();
    }

    pub fn create_window_swap_chain(&mut self, device: &wgpu::Device, window: &Window) {
        let swap_chain_descriptor: wgpu::SwapChainDescriptor = window.wgpu_into();
        let surface = self
            .window_surfaces
            .get(&window.id)
            .expect("No surface found for window");
        let swap_chain = device.create_swap_chain(surface, &swap_chain_descriptor);
        self.window_swap_chains.insert(window.id, swap_chain);
    }

    pub fn add_resource_info(&mut self, resource: RenderResource, resource_info: ResourceInfo) {
        self.resource_info.insert(resource, resource_info);
    }

    pub fn get_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        render_resource_set_id: RenderResourceSetId,
    ) -> Option<&wgpu::BindGroup> {
        if let Some(bind_group_info) = self.bind_groups.get(&bind_group_descriptor_id) {
            bind_group_info.bind_groups.get(&render_resource_set_id)
        } else {
            None
        }
    }

    pub fn create_bind_group(
        &self,
        device: &wgpu::Device,
        render_resource_set_id: RenderResourceSetId,
        bind_group_descriptor: &wgpu::BindGroupDescriptor,
    ) -> wgpu::BindGroup {
        log::trace!(
            "created bind group for RenderResourceSet {:?}",
            render_resource_set_id
        );
        log::trace!("{:#?}", bind_group_descriptor);
        device.create_bind_group(bind_group_descriptor)
    }

    pub fn set_bind_group(
        &mut self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        render_resource_set_id: RenderResourceSetId,
        bind_group: wgpu::BindGroup,
    ) {
        let bind_group_info = self
            .bind_groups
            .entry(bind_group_descriptor_id)
            .or_insert_with(|| WgpuBindGroupInfo::default());
        bind_group_info
            .bind_groups
            .insert(render_resource_set_id, bind_group);
    }

    pub fn create_buffer(
        &mut self,
        device: &wgpu::Device,
        buffer_info: BufferInfo,
    ) -> RenderResource {
        let buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: None,
            size: buffer_info.size as u64,
            usage: buffer_info.buffer_usage.wgpu_into(),
        });

        let resource = RenderResource::new();
        self.add_resource_info(resource, ResourceInfo::Buffer(buffer_info));

        self.buffers.insert(resource, buffer);
        resource
    }

    pub fn create_buffer_with_data(
        &mut self,
        device: &wgpu::Device,
        mut buffer_info: BufferInfo,
        data: &[u8],
    ) -> RenderResource {
        buffer_info.size = data.len();
        let buffer = device.create_buffer_with_data(data, buffer_info.buffer_usage.wgpu_into());
        self.assign_buffer(buffer, buffer_info)
    }

    pub fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo> {
        self.resource_info.get(&resource)
    }

    pub fn remove_buffer(&mut self, resource: RenderResource) {
        self.buffers.remove(&resource);
        self.resource_info.remove(&resource);
    }

    pub fn assign_buffer(
        &mut self,
        buffer: wgpu::Buffer,
        buffer_info: BufferInfo,
    ) -> RenderResource {
        let resource = RenderResource::new();
        self.add_resource_info(resource, ResourceInfo::Buffer(buffer_info));
        self.buffers.insert(resource, buffer);
        resource
    }

    // TODO: clean this up
    pub fn begin_create_buffer_mapped_render_context(
        buffer_info: &BufferInfo,
        render_resources: &mut WgpuRenderResourceContext,
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn RenderResourceContext),
    ) -> wgpu::Buffer {
        let device = render_resources.device.clone();
        let mut mapped = device.create_buffer_mapped(&wgpu::BufferDescriptor {
            size: buffer_info.size as u64,
            usage: buffer_info.buffer_usage.wgpu_into(),
            label: None,
        });
        setup_data(&mut mapped.data, render_resources);
        mapped.finish()
    }

    pub fn begin_create_buffer_mapped_transactional_render_context(
        buffer_info: &BufferInfo,
        render_resources: &mut WgpuTransactionalRenderResourceContext,
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn RenderResourceContext),
    ) -> wgpu::Buffer {
        let device = render_resources.device.clone();
        let mut mapped = device.create_buffer_mapped(&wgpu::BufferDescriptor {
            size: buffer_info.size as u64,
            usage: buffer_info.buffer_usage.wgpu_into(),
            label: None,
        });
        setup_data(&mut mapped.data, render_resources);
        mapped.finish()
    }

    pub fn copy_buffer_to_buffer(
        &mut self,
        encoder: &mut wgpu::CommandEncoder,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    ) {
        let source = self.buffers.get(&source_buffer).unwrap();
        let destination = self.buffers.get(&destination_buffer).unwrap();
        encoder.copy_buffer_to_buffer(source, source_offset, destination, destination_offset, size);
    }

    pub fn create_shader_module(
        &mut self,
        device: &wgpu::Device,
        shader_handle: Handle<Shader>,
        shader: &Shader,
    ) {
        let shader_module = device.create_shader_module(&shader.get_spirv(None));
        self.shader_modules.insert(shader_handle, shader_module);
    }

    pub fn create_sampler(
        &mut self,
        device: &wgpu::Device,
        sampler_descriptor: &SamplerDescriptor,
    ) -> RenderResource {
        let descriptor: wgpu::SamplerDescriptor = (*sampler_descriptor).wgpu_into();
        let sampler = device.create_sampler(&descriptor);
        let resource = RenderResource::new();
        self.samplers.insert(resource, sampler);
        self.add_resource_info(resource, ResourceInfo::Sampler);
        resource
    }

    pub fn create_texture(
        &mut self,
        device: &wgpu::Device,
        texture_descriptor: &TextureDescriptor,
    ) -> RenderResource {
        let descriptor: wgpu::TextureDescriptor = (*texture_descriptor).wgpu_into();
        let texture = device.create_texture(&descriptor);
        let texture_view = texture.create_default_view();
        let resource = RenderResource::new();
        self.add_resource_info(resource, ResourceInfo::Texture);
        self.textures.insert(resource, texture_view);
        resource
    }

    pub fn create_texture_with_data(
        &mut self,
        device: &wgpu::Device,
        encoder: &mut wgpu::CommandEncoder,
        texture_descriptor: &TextureDescriptor,
        bytes: &[u8],
    ) -> RenderResource {
        let descriptor: wgpu::TextureDescriptor = (*texture_descriptor).wgpu_into();
        let texture = device.create_texture(&descriptor);
        let texture_view = texture.create_default_view();
        let temp_buf = device.create_buffer_with_data(bytes, wgpu::BufferUsage::COPY_SRC);
        encoder.copy_buffer_to_texture(
            wgpu::BufferCopyView {
                buffer: &temp_buf,
                offset: 0,
                bytes_per_row: 4 * descriptor.size.width,
                rows_per_image: 0, // NOTE: Example sets this to 0, but should it be height?
            },
            wgpu::TextureCopyView {
                texture: &texture,
                mip_level: 0,
                array_layer: 0,
                origin: wgpu::Origin3d { x: 0, y: 0, z: 0 },
            },
            descriptor.size,
        );

        let resource = RenderResource::new();
        self.add_resource_info(resource, ResourceInfo::Texture);
        self.textures.insert(resource, texture_view);
        resource
    }

    pub fn remove_texture(&mut self, resource: RenderResource) {
        self.textures.remove(&resource);
        self.resource_info.remove(&resource);
    }

    pub fn remove_sampler(&mut self, resource: RenderResource) {
        self.samplers.remove(&resource);
        self.resource_info.remove(&resource);
    }

    pub fn get_render_resources(&self) -> &AssetResources {
        &self.asset_resources
    }

    pub fn get_render_resources_mut(&mut self) -> &mut AssetResources {
        &mut self.asset_resources
    }

    pub fn create_bind_group_layout(
        &mut self,
        device: &wgpu::Device,
        bind_group_id: BindGroupDescriptorId,
        descriptor: &wgpu::BindGroupLayoutDescriptor,
    ) {
        let wgpu_bind_group_layout = device.create_bind_group_layout(descriptor);
        self.bind_group_layouts
            .insert(bind_group_id, wgpu_bind_group_layout);
    }

    pub fn create_render_pipeline(
        &self,
        device: &wgpu::Device,
        descriptor: &wgpu::RenderPipelineDescriptor,
    ) -> wgpu::RenderPipeline {
        device.create_render_pipeline(&descriptor)
    }

    pub fn set_render_pipeline(
        &mut self,
        pipeline_handle: Handle<PipelineDescriptor>,
        render_pipeline: wgpu::RenderPipeline,
    ) {
        self.render_pipelines
            .insert(pipeline_handle, render_pipeline);
    }
}
