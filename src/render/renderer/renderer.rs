use crate::{
    legion::prelude::*,
    render::{
        pipeline::PipelineDescriptor,
        render_resource::{
            BufferInfo, RenderResource, RenderResourceAssignments, RenderResources, ResourceInfo,
        },
        texture::{SamplerDescriptor, TextureDescriptor},
    },
};
use std::ops::Range;

pub trait Renderer {
    fn resize(&mut self, world: &mut World, resources: &mut Resources, width: u32, height: u32);
    fn update(&mut self, world: &mut World, resources: &mut Resources);
    fn create_buffer_with_data(&mut self, buffer_info: BufferInfo, data: &[u8]) -> RenderResource;
    fn create_sampler(&mut self, sampler_descriptor: &SamplerDescriptor) -> RenderResource;
    fn create_texture(
        &mut self,
        texture_descriptor: &TextureDescriptor,
        bytes: Option<&[u8]>,
    ) -> RenderResource;
    fn create_buffer(&mut self, buffer_info: BufferInfo) -> RenderResource;
    fn create_buffer_mapped(
        &mut self,
        buffer_info: BufferInfo,
        func: &mut dyn FnMut(&mut [u8], &mut dyn Renderer),
    ) -> RenderResource;
    fn remove_buffer(&mut self, resource: RenderResource);
    fn remove_texture(&mut self, resource: RenderResource);
    fn remove_sampler(&mut self, resource: RenderResource);
    fn get_resource_info(&self, resource: RenderResource) -> Option<&ResourceInfo>;
    fn get_resource_info_mut(&mut self, resource: RenderResource) -> Option<&mut ResourceInfo>;
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
    fn setup_bind_groups(
        &mut self,
        render_resource_assignments: &RenderResourceAssignments,
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
    fn set_render_resource_assignments(&mut self, render_resource_assignments: Option<&RenderResourceAssignments>) -> Option<Range<u32>>;
}
