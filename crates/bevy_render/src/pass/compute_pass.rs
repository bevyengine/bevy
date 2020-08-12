use crate::{
    pipeline::{BindGroupDescriptorId, ComputePipelineDescriptor},
    renderer::{BindGroupId, RenderContext},
};
use bevy_asset::Handle;

pub trait ComputePass {
    fn get_render_context(&self) -> &dyn RenderContext;
    fn set_pipeline(&mut self, pipeline_handle: Handle<ComputePipelineDescriptor>);
    fn set_bind_group(
        &mut self,
        index: u32,
        bind_group_descriptor_id: BindGroupDescriptorId,
        bind_group: BindGroupId,
        dynamic_uniform_indices: Option<&[u32]>,
    );
    fn dispatch(&mut self, x: u32, y: u32, z: u32);
}
