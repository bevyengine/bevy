use crate::WgpuResources;

use bevy_asset::{AssetStorage, Handle};
use bevy_render::{
    mesh::Mesh,
    pipeline::{BindGroupDescriptorId, PipelineDescriptor},
    render_resource::{
        AssetResources, BufferInfo, RenderResource, RenderResourceSetId, ResourceInfo,
    },
    renderer_2::RenderResourceContext,
    shader::Shader,
    texture::{SamplerDescriptor, Texture, TextureDescriptor},
};
use std::sync::Arc;
use bevy_window::{WindowId, Window};

pub struct WgpuRenderResourceContext {
    pub device: Arc<wgpu::Device>,
    pub wgpu_resources: WgpuResources,
}

// TODO: make this name not terrible
pub trait WgpuRenderResourceContextTrait {
    fn create_texture_with_data(
        &mut self,
        command_encoder: &mut wgpu::CommandEncoder,
        texture_descriptor: &TextureDescriptor,
        bytes: &[u8],
    ) -> RenderResource;

    fn create_bind_group(
        &self,
        render_resource_set_id: RenderResourceSetId,
        descriptor: &wgpu::BindGroupDescriptor,
    ) -> wgpu::BindGroup;
    fn set_bind_group(
        &mut self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        render_resource_set_id: RenderResourceSetId,
        bind_group: wgpu::BindGroup,
    );
    fn create_bind_group_layout(
        &mut self,
        bind_group_id: BindGroupDescriptorId,
        descriptor: &wgpu::BindGroupLayoutDescriptor,
    );
    fn create_render_pipeline(
        &self,
        descriptor: &wgpu::RenderPipelineDescriptor,
    ) -> wgpu::RenderPipeline;
    fn set_render_pipeline(
        &mut self,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline: wgpu::RenderPipeline,
    );
    fn get_bind_group(
        &self,
        bind_group_id: BindGroupDescriptorId,
        render_resource_set_id: RenderResourceSetId,
    ) -> Option<&wgpu::BindGroup>;
    fn get_bind_group_layout(
        &self,
        bind_group_id: BindGroupDescriptorId,
    ) -> Option<&wgpu::BindGroupLayout>;
    fn get_buffer(&self, render_resource: RenderResource) -> Option<&wgpu::Buffer>;
    fn get_swap_chain_output(&self, window_id: &WindowId) -> Option<&wgpu::SwapChainOutput>;
    fn get_texture(&self, render_resource: RenderResource) -> Option<&wgpu::TextureView>;
    fn get_sampler(&self, render_resource: RenderResource) -> Option<&wgpu::Sampler>;
    fn get_pipeline(&self, pipeline: Handle<PipelineDescriptor>) -> Option<&wgpu::RenderPipeline>;
    fn get_shader_module(&self, shader: Handle<Shader>) -> Option<&wgpu::ShaderModule>;
}

impl WgpuRenderResourceContextTrait for WgpuRenderResourceContext {
    fn get_buffer(&self, render_resource: RenderResource) -> Option<&wgpu::Buffer> {
        self.wgpu_resources.buffers.get(&render_resource)
    }
    fn create_texture_with_data(
        &mut self,
        command_encoder: &mut wgpu::CommandEncoder,
        texture_descriptor: &TextureDescriptor,
        bytes: &[u8],
    ) -> RenderResource {
        self.wgpu_resources.create_texture_with_data(
            &self.device,
            command_encoder,
            texture_descriptor,
            bytes,
        )
    }
    fn create_bind_group(
        &self,
        render_resource_set_id: RenderResourceSetId,
        descriptor: &wgpu::BindGroupDescriptor,
    ) -> wgpu::BindGroup {
        self.wgpu_resources
            .create_bind_group(&self.device, render_resource_set_id, descriptor)
    }
    fn create_bind_group_layout(
        &mut self,
        bind_group_id: BindGroupDescriptorId,
        descriptor: &wgpu::BindGroupLayoutDescriptor,
    ) {
        self.wgpu_resources
            .create_bind_group_layout(&self.device, bind_group_id, descriptor);
    }
    fn create_render_pipeline(
        &self,
        descriptor: &wgpu::RenderPipelineDescriptor,
    ) -> wgpu::RenderPipeline {
        self.wgpu_resources
            .create_render_pipeline(&self.device, descriptor)
    }
    fn get_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        render_resource_set_id: RenderResourceSetId,
    ) -> Option<&wgpu::BindGroup> {
        self.wgpu_resources
            .get_bind_group(bind_group_descriptor_id, render_resource_set_id)
    }
    fn get_bind_group_layout(
        &self,
        bind_group_id: BindGroupDescriptorId,
    ) -> Option<&wgpu::BindGroupLayout> {
        self.wgpu_resources.bind_group_layouts.get(&bind_group_id)
    }
    fn get_texture(&self, render_resource: RenderResource) -> Option<&wgpu::TextureView> {
        self.wgpu_resources.textures.get(&render_resource)
    }
    fn get_sampler(&self, render_resource: RenderResource) -> Option<&wgpu::Sampler> {
        self.wgpu_resources.samplers.get(&render_resource)
    }
    fn get_pipeline(&self, pipeline: Handle<PipelineDescriptor>) -> Option<&wgpu::RenderPipeline> {
        self.wgpu_resources.render_pipelines.get(&pipeline)
    }
    fn get_shader_module(&self, shader: Handle<Shader>) -> Option<&wgpu::ShaderModule> {
        self.wgpu_resources.shader_modules.get(&shader)
    }
    fn set_bind_group(
        &mut self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        render_resource_set_id: RenderResourceSetId,
        bind_group: wgpu::BindGroup,
    ) {
        self.wgpu_resources.set_bind_group(
            bind_group_descriptor_id,
            render_resource_set_id,
            bind_group,
        );
    }
    fn set_render_pipeline(
        &mut self,
        pipeline_handle: Handle<PipelineDescriptor>,
        pipeline: wgpu::RenderPipeline,
    ) {
        self.wgpu_resources
            .set_render_pipeline(pipeline_handle, pipeline);
    }
    fn get_swap_chain_output(&self, window_id: &WindowId) -> Option<&wgpu::SwapChainOutput> {
        self.wgpu_resources.swap_chain_outputs.get(window_id)
    }
}

impl WgpuRenderResourceContext {
    pub fn new(device: Arc<wgpu::Device>) -> Self {
        WgpuRenderResourceContext {
            device,
            wgpu_resources: WgpuResources::default(),
        }
    }
}

impl RenderResourceContext for WgpuRenderResourceContext {
    fn create_sampler(&mut self, sampler_descriptor: &SamplerDescriptor) -> RenderResource {
        self.wgpu_resources
            .create_sampler(&self.device, sampler_descriptor)
    }
    fn create_texture(&mut self, texture_descriptor: &TextureDescriptor) -> RenderResource {
        self.wgpu_resources
            .create_texture(&self.device, texture_descriptor)
    }
    fn create_buffer(&mut self, buffer_info: BufferInfo) -> RenderResource {
        self.wgpu_resources.create_buffer(&self.device, buffer_info)
    }

    // TODO: clean this up
    fn create_buffer_mapped(
        &mut self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn RenderResourceContext),
    ) -> RenderResource {
        let buffer = WgpuResources::begin_create_buffer_mapped_render_context(
            &buffer_info,
            self,
            setup_data,
        );
        self.wgpu_resources.assign_buffer(buffer, buffer_info)
    }

    fn create_buffer_with_data(&mut self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource {
        self.wgpu_resources
            .create_buffer_with_data(&self.device, buffer_info, data)
    }

    fn remove_buffer(&mut self, resource: RenderResource) {
        self.wgpu_resources.remove_buffer(resource);
    }
    fn remove_texture(&mut self, resource: RenderResource) {
        self.wgpu_resources.remove_texture(resource);
    }
    fn remove_sampler(&mut self, resource: RenderResource) {
        self.wgpu_resources.remove_sampler(resource);
    }

    fn get_texture_resource(&self, texture: Handle<Texture>) -> Option<RenderResource> {
        self.wgpu_resources
            .asset_resources
            .get_texture_resource(texture)
    }

    fn get_texture_sampler_resource(&self, texture: Handle<Texture>) -> Option<RenderResource> {
        self.wgpu_resources
            .asset_resources
            .get_texture_sampler_resource(texture)
    }

    fn get_mesh_vertices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource> {
        self.wgpu_resources
            .asset_resources
            .get_mesh_vertices_resource(mesh)
    }

    fn get_mesh_indices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource> {
        self.wgpu_resources
            .asset_resources
            .get_mesh_indices_resource(mesh)
    }

    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo> {
        self.wgpu_resources.get_resource_info(resource)
    }

    fn asset_resources(&self) -> &AssetResources {
        &self.wgpu_resources.asset_resources
    }
    fn asset_resources_mut(&mut self) -> &mut AssetResources {
        &mut self.wgpu_resources.asset_resources
    }
    fn create_shader_module(
        &mut self,
        shader_handle: Handle<Shader>,
        shader_storage: &AssetStorage<Shader>,
    ) {
        if self.get_shader_module(shader_handle).is_some() {
            return;
        }

        let shader = shader_storage.get(&shader_handle).unwrap();
        self.wgpu_resources
            .create_shader_module(&self.device, shader_handle, shader);
    }
    fn create_swap_chain(&mut self, window: &Window) {
        self.wgpu_resources.create_window_swap_chain(&self.device, window)
    }
    fn next_swap_chain_texture(&mut self, window_id: bevy_window::WindowId) {
        self.wgpu_resources.next_swap_chain_texture(window_id);
    }
    fn drop_swap_chain_texture(&mut self, window_id: WindowId) {
        self.wgpu_resources.remove_swap_chain_texture(window_id);
    }
}
