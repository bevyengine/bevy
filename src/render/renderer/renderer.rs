use crate::{
    legion::prelude::*,
    render::{
        pipeline::PipelineDescriptor,
        render_graph::RenderGraph,
        render_resource::{BufferUsage, RenderResource, RenderResources, ResourceInfo},
        shader::DynamicUniformBufferInfo,
        texture::{SamplerDescriptor, TextureDescriptor},
    },
};
use std::ops::Range;

pub trait Renderer {
    fn initialize(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
        render_graph: &mut RenderGraph,
    );
    fn resize(
        &mut self,
        world: &mut World,
        resources: &mut Resources,
        render_graph: &mut RenderGraph,
        width: u32,
        height: u32,
    );
    fn process_render_graph(
        &mut self,
        render_graph: &mut RenderGraph,
        world: &mut World,
        resources: &mut Resources,
    );
    fn create_buffer_with_data(&mut self, data: &[u8], buffer_usage: BufferUsage)
        -> RenderResource;
    fn create_sampler(&mut self, sampler_descriptor: &SamplerDescriptor) -> RenderResource;
    fn create_texture(
        &mut self,
        texture_descriptor: &TextureDescriptor,
        bytes: Option<&[u8]>,
    ) -> RenderResource;
    // TODO: remove this and replace it with ResourceInfo
    fn get_dynamic_uniform_buffer_info(
        &self,
        resource: RenderResource,
    ) -> Option<&DynamicUniformBufferInfo>;
    fn get_dynamic_uniform_buffer_info_mut(
        &mut self,
        resource: RenderResource,
    ) -> Option<&mut DynamicUniformBufferInfo>;
    fn add_dynamic_uniform_buffer_info(
        &mut self,
        resource: RenderResource,
        info: DynamicUniformBufferInfo,
    );
    fn create_buffer(&mut self, size: u64, buffer_usage: BufferUsage) -> RenderResource;
    fn create_instance_buffer(
        &mut self,
        mesh_id: usize,
        size: usize,
        count: usize,
        buffer_usage: BufferUsage,
    ) -> RenderResource;
    fn create_instance_buffer_with_data(
        &mut self,
        mesh_id: usize,
        data: &[u8],
        size: usize,
        count: usize,
        buffer_usage: BufferUsage,
    ) -> RenderResource;
    fn create_buffer_mapped(
        &mut self,
        size: usize,
        buffer_usage: BufferUsage,
        func: &mut dyn FnMut(&mut [u8]),
    ) -> RenderResource;
    fn remove_buffer(&mut self, resource: RenderResource);
    fn remove_texture(&mut self, resource: RenderResource);
    fn remove_sampler(&mut self, resource: RenderResource);
    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo>;
    fn copy_buffer_to_buffer(
        &mut self,
        source_buffer: RenderResource,
        source_offset: u64,
        destination_buffer: RenderResource,
        destination_offset: u64,
        size: u64,
    );
    fn get_render_resources(&self) -> &RenderResources;
    fn get_render_resources_mut(&mut self) -> &mut RenderResources;
    fn set_entity_uniform_resource(
        &mut self,
        entity: Entity,
        uniform_name: &str,
        resource: RenderResource,
    );
    fn get_entity_uniform_resource(
        &self,
        entity: Entity,
        uniform_name: &str,
    ) -> Option<RenderResource>;
    fn setup_entity_bind_groups(
        &mut self,
        entity: Entity,
        pipeline_descriptor: &PipelineDescriptor,
    );
}

pub trait RenderPass {
    // TODO: consider using static dispatch for the renderer: Renderer<WgpuBackend>. compare compile times
    fn get_renderer(&self) -> &dyn Renderer;
    fn get_pipeline_descriptor(&self) -> &PipelineDescriptor;
    fn set_index_buffer(&mut self, resource: RenderResource, offset: u64);
    fn set_vertex_buffer(&mut self, start_slot: u32, resource: RenderResource, offset: u64);
    fn draw_indexed(&mut self, indices: Range<u32>, base_vertex: i32, instances: Range<u32>);
    fn set_bind_groups(&mut self, entity: Option<&Entity>);
}
