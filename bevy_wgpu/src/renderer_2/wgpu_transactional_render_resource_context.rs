use crate::WgpuResources;

use super::WgpuRenderResourceContextTrait;
use bevy_asset::Handle;
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

pub struct WgpuTransactionalRenderResourceContext<'a> {
    pub device: Arc<wgpu::Device>,
    pub local_resources: WgpuResources,
    pub parent_resources: &'a WgpuResources,
    removed_resources: Vec<RenderResource>,
}

impl<'a> WgpuRenderResourceContextTrait for WgpuTransactionalRenderResourceContext<'a> {
    fn get_buffer(&self, render_resource: RenderResource) -> Option<&wgpu::Buffer> {
        let local = self.local_resources.buffers.get(&render_resource);
        if local.is_some() {
            return local;
        }

        self.parent_resources.buffers.get(&render_resource)
    }

    fn create_texture_with_data(
        &mut self,
        command_encoder: &mut wgpu::CommandEncoder,
        texture_descriptor: &TextureDescriptor,
        bytes: &[u8],
    ) -> RenderResource {
        self.local_resources.create_texture_with_data(
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
        self.local_resources
            .create_bind_group(&self.device, render_resource_set_id, descriptor)
    }
    fn create_bind_group_layout(
        &mut self,
        bind_group_id: BindGroupDescriptorId,
        descriptor: &wgpu::BindGroupLayoutDescriptor,
    ) {
        self.local_resources
            .create_bind_group_layout(&self.device, bind_group_id, descriptor);
    }
    fn create_render_pipeline(
        &self,
        descriptor: &wgpu::RenderPipelineDescriptor,
    ) -> wgpu::RenderPipeline {
        self.local_resources
            .create_render_pipeline(&self.device, descriptor)
    }
    fn get_bind_group(
        &self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        render_resource_set_id: RenderResourceSetId,
    ) -> Option<&wgpu::BindGroup> {
        let local = self
            .local_resources
            .get_bind_group(bind_group_descriptor_id, render_resource_set_id);
        if local.is_some() {
            return local;
        }
        self.parent_resources
            .get_bind_group(bind_group_descriptor_id, render_resource_set_id)
    }
    fn get_bind_group_layout(
        &self,
        bind_group_id: BindGroupDescriptorId,
    ) -> Option<&wgpu::BindGroupLayout> {
        let local = self.local_resources.bind_group_layouts.get(&bind_group_id);
        if local.is_some() {
            return local;
        }
        self.parent_resources.bind_group_layouts.get(&bind_group_id)
    }
    fn get_texture(&self, render_resource: RenderResource) -> Option<&wgpu::TextureView> {
        let local = self.local_resources.textures.get(&render_resource);
        if local.is_some() {
            return local;
        }
        self.parent_resources.textures.get(&render_resource)
    }
    fn get_sampler(&self, render_resource: RenderResource) -> Option<&wgpu::Sampler> {
        let local = self.local_resources.samplers.get(&render_resource);
        if local.is_some() {
            return local;
        }
        self.parent_resources.samplers.get(&render_resource)
    }
    fn get_pipeline(&self, pipeline: Handle<PipelineDescriptor>) -> Option<&wgpu::RenderPipeline> {
        let local = self.local_resources.render_pipelines.get(&pipeline);
        if local.is_some() {
            return local;
        }
        self.parent_resources.render_pipelines.get(&pipeline)
    }
    fn get_shader_module(&self, shader: Handle<Shader>) -> Option<&wgpu::ShaderModule> {
        let local = self.local_resources.shader_modules.get(&shader);
        if local.is_some() {
            return local;
        }
        self.parent_resources.shader_modules.get(&shader)
    }
    fn set_bind_group(
        &mut self,
        bind_group_descriptor_id: BindGroupDescriptorId,
        render_resource_set_id: RenderResourceSetId,
        bind_group: wgpu::BindGroup,
    ) {
        self.local_resources.set_bind_group(
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
        self.local_resources
            .set_render_pipeline(pipeline_handle, pipeline);
    }
}

impl<'a> WgpuTransactionalRenderResourceContext<'a> {
    pub fn new(device: Arc<wgpu::Device>, parent_resources: &'a WgpuResources) -> Self {
        WgpuTransactionalRenderResourceContext {
            device,
            local_resources: WgpuResources::default(),
            parent_resources,
            removed_resources: Vec::new(),
        }
    }
}

impl<'a> RenderResourceContext for WgpuTransactionalRenderResourceContext<'a> {
    fn create_sampler(&mut self, sampler_descriptor: &SamplerDescriptor) -> RenderResource {
        self.local_resources
            .create_sampler(&self.device, sampler_descriptor)
    }
    fn create_texture(&mut self, texture_descriptor: &TextureDescriptor) -> RenderResource {
        self.local_resources
            .create_texture(&self.device, texture_descriptor)
    }
    fn create_buffer(&mut self, buffer_info: BufferInfo) -> RenderResource {
        self.local_resources
            .create_buffer(&self.device, buffer_info)
    }

    // TODO: clean this up
    fn create_buffer_mapped(
        &mut self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &mut dyn RenderResourceContext),
    ) -> RenderResource {
        let buffer = WgpuResources::begin_create_buffer_mapped_transactional_render_context(
            &buffer_info,
            self,
            setup_data,
        );
        self.local_resources.assign_buffer(buffer, buffer_info)
    }

    fn create_buffer_with_data(&mut self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource {
        self.local_resources
            .create_buffer_with_data(&self.device, buffer_info, data)
    }

    fn remove_buffer(&mut self, resource: RenderResource) {
        self.local_resources.remove_buffer(resource);
        self.removed_resources.push(resource);
    }
    fn remove_texture(&mut self, resource: RenderResource) {
        self.local_resources.remove_texture(resource);
        self.removed_resources.push(resource);
    }
    fn remove_sampler(&mut self, resource: RenderResource) {
        self.local_resources.remove_sampler(resource);
        self.removed_resources.push(resource);
    }

    fn get_texture_resource(&self, texture: Handle<Texture>) -> Option<RenderResource> {
        let local = self
            .local_resources
            .asset_resources
            .get_texture_resource(texture);
        if local.is_some() {
            return local;
        }

        self.parent_resources
            .asset_resources
            .get_texture_resource(texture)
    }

    fn get_texture_sampler_resource(&self, texture: Handle<Texture>) -> Option<RenderResource> {
        let local = self
            .local_resources
            .asset_resources
            .get_texture_sampler_resource(texture);

        if local.is_some() {
            return local;
        }

        self.parent_resources
            .asset_resources
            .get_texture_sampler_resource(texture)
    }

    fn get_mesh_vertices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource> {
        let local = self
            .local_resources
            .asset_resources
            .get_mesh_vertices_resource(mesh);
        if local.is_some() {
            return local;
        }

        self.parent_resources
            .asset_resources
            .get_mesh_vertices_resource(mesh)
    }

    fn get_mesh_indices_resource(&self, mesh: Handle<Mesh>) -> Option<RenderResource> {
        let local = self
            .local_resources
            .asset_resources
            .get_mesh_indices_resource(mesh);
        if local.is_some() {
            return local;
        }

        self.parent_resources
            .asset_resources
            .get_mesh_indices_resource(mesh)
    }

    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo> {
        let local = self.local_resources.get_resource_info(resource);
        if local.is_some() {
            return local;
        }

        self.parent_resources.get_resource_info(resource)
    }

    fn asset_resources(&self) -> &AssetResources {
        &self.local_resources.asset_resources
    }
    fn asset_resources_mut(&mut self) -> &mut AssetResources {
        &mut self.local_resources.asset_resources
    }
    fn create_shader_module(
        &mut self,
        shader_handle: Handle<Shader>,
        shader_storage: &bevy_asset::AssetStorage<Shader>,
    ) {
        if self.get_shader_module(shader_handle).is_some() {
            return;
        }

        let shader = shader_storage.get(&shader_handle).unwrap();
        self.local_resources
            .create_shader_module(&self.device, shader_handle, shader);
    }
}
