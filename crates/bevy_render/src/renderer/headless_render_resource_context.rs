use super::RenderResourceContext;
use crate::{
    pipeline::{BindGroupDescriptor, PipelineDescriptor},
    render_resource::{
        BufferInfo, RenderResourceId, RenderResourceAssignments, RenderResourceSetId, ResourceInfo,
    },
    shader::Shader,
    texture::{SamplerDescriptor, TextureDescriptor},
};
use bevy_asset::{Assets, Handle, HandleUntyped};
use bevy_window::{Window, WindowId};
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default)]
pub struct HeadlessRenderResourceContext {
    resource_info: Arc<RwLock<HashMap<RenderResourceId, ResourceInfo>>>,
    pub asset_resources: Arc<RwLock<HashMap<(HandleUntyped, usize), RenderResourceId>>>,
}

impl HeadlessRenderResourceContext {
    pub fn add_resource_info(&self, resource: RenderResourceId, resource_info: ResourceInfo) {
        self.resource_info
            .write()
            .unwrap()
            .insert(resource, resource_info);
    }
}

impl RenderResourceContext for HeadlessRenderResourceContext {
    fn create_swap_chain(&self, _window: &Window) {}
    fn next_swap_chain_texture(&self, _window_id: WindowId) -> RenderResourceId {
        RenderResourceId::new()
    }
    fn drop_swap_chain_texture(&self, _render_resource: RenderResourceId) {}
    fn drop_all_swap_chain_textures(&self) {}
    fn create_sampler(&self, _sampler_descriptor: &SamplerDescriptor) -> RenderResourceId {
        let resource = RenderResourceId::new();
        self.add_resource_info(resource, ResourceInfo::Sampler);
        resource
    }
    fn create_texture(&self, texture_descriptor: TextureDescriptor) -> RenderResourceId {
        let resource = RenderResourceId::new();
        self.add_resource_info(resource, ResourceInfo::Texture(Some(texture_descriptor)));
        resource
    }
    fn create_buffer(&self, buffer_info: BufferInfo) -> RenderResourceId {
        let resource = RenderResourceId::new();
        self.add_resource_info(resource, ResourceInfo::Buffer(Some(buffer_info)));
        resource
    }
    fn create_buffer_mapped(
        &self,
        buffer_info: BufferInfo,
        setup_data: &mut dyn FnMut(&mut [u8], &dyn RenderResourceContext),
    ) -> RenderResourceId {
        let mut buffer = vec![0; buffer_info.size];
        setup_data(&mut buffer, self);
        RenderResourceId::new()
    }
    fn create_buffer_with_data(&self, buffer_info: BufferInfo, _data: &[u8]) -> RenderResourceId {
        let resource = RenderResourceId::new();
        self.add_resource_info(resource, ResourceInfo::Buffer(Some(buffer_info)));
        resource
    }
    fn create_shader_module(&self, _shader_handle: Handle<Shader>, _shaders: &Assets<Shader>) {}
    fn remove_buffer(&self, resource: RenderResourceId) {
        self.resource_info.write().unwrap().remove(&resource);
    }
    fn remove_texture(&self, resource: RenderResourceId) {
        self.resource_info.write().unwrap().remove(&resource);
    }
    fn remove_sampler(&self, resource: RenderResourceId) {
        self.resource_info.write().unwrap().remove(&resource);
    }
    fn get_resource_info(
        &self,
        resource: RenderResourceId,
        handle_info: &mut dyn FnMut(Option<&ResourceInfo>),
    ) {
        handle_info(self.resource_info.read().unwrap().get(&resource));
    }
    fn set_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        render_resource: RenderResourceId,
        index: usize,
    ) {
        self.asset_resources
            .write()
            .unwrap()
            .insert((handle, index), render_resource);
    }
    fn get_asset_resource_untyped(
        &self,
        handle: HandleUntyped,
        index: usize,
    ) -> Option<RenderResourceId> {
        self.asset_resources
            .write()
            .unwrap()
            .get(&(handle, index))
            .cloned()
    }
    fn create_render_pipeline(
        &self,
        _pipeline_handle: Handle<PipelineDescriptor>,
        _pipeline_descriptor: &PipelineDescriptor,
        _shaders: &Assets<Shader>,
    ) {
    }
    fn create_bind_group(
        &self,
        bind_group_descriptor: &BindGroupDescriptor,
        render_resource_assignments: &RenderResourceAssignments,
    ) -> Option<RenderResourceSetId> {
        if let Some(resource_set) =
            render_resource_assignments.get_render_resource_set(bind_group_descriptor.id)
        {
            Some(resource_set.id)
        } else {
            None
        }
    }
    fn create_shader_module_from_source(&self, _shader_handle: Handle<Shader>, _shader: &Shader) {}
    fn remove_asset_resource_untyped(&self, handle: HandleUntyped, index: usize) {
        self.asset_resources
            .write()
            .unwrap()
            .remove(&(handle, index));
    }
    fn clear_bind_groups(&self) {}
}
