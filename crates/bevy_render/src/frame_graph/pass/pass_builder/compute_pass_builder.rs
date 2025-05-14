use std::mem::take;

use tracing::warn;
use wgpu::QuerySet;

use crate::{
    frame_graph::{
        BindGroupDrawing, ComputePass, ComputePassCommandBuilder, FrameGraphBuffer,
        PassNodeBuilder, ResourceHandle, ResourceMaterial, ResourceRead, ResourceRef,
        ResourceWrite,
    },
    render_resource::{BindGroup, CachedComputePipelineId},
};

use super::PassBuilder;

pub struct ComputePassBuilder<'a, 'b> {
    compute_pass: ComputePass,
    pass_builder: &'b mut PassBuilder<'a>,
}

impl<'a, 'b> Drop for ComputePassBuilder<'a, 'b> {
    fn drop(&mut self) {
        self.finish();
    }
}

impl<'a, 'b> ComputePassBuilder<'a, 'b> {
    pub fn new(pass_builder: &'b mut PassBuilder<'a>) -> Self {
        let compute_pass = ComputePass::default();

        Self {
            compute_pass,
            pass_builder,
        }
    }

    pub fn read_material<M: ResourceMaterial>(
        &mut self,
        material: &M,
    ) -> ResourceRef<M::ResourceType, ResourceRead> {
        self.pass_builder.read_material(material)
    }

    pub fn write_material<M: ResourceMaterial>(
        &mut self,
        material: &M,
    ) -> ResourceRef<M::ResourceType, ResourceWrite> {
        self.pass_builder.write_material(material)
    }

    pub fn dispatch_workgroups_indirect(
        &mut self,
        indirect_buffer_ref: &ResourceRef<FrameGraphBuffer, ResourceRead>,
        indirect_offset: u64,
    ) -> &mut Self {
        self.compute_pass
            .dispatch_workgroups_indirect(indirect_buffer_ref, indirect_offset);
        self
    }

    pub fn set_push_constants(&mut self, offset: u32, data: &[u8]) -> &mut Self {
        self.compute_pass.set_push_constants(offset, data);

        self
    }

    pub fn set_pass_name(&mut self, name: &str) -> &mut Self {
        self.compute_pass.set_pass_name(name);

        self
    }

    fn finish(&mut self) {
        self.compute_pass.finish();

        let compute_pass = take(&mut self.compute_pass);

        if compute_pass.is_vaild() {
            self.pass_builder.add_executor(compute_pass);
        } else {
            warn!("{:?} compute pass must is vaild", compute_pass.pass_name());
        }
    }

    pub fn dispatch_workgroups(&mut self, x: u32, y: u32, z: u32) -> &mut Self {
        self.compute_pass.dispatch_workgroups(x, y, z);

        self
    }

    pub fn set_compute_pipeline(&mut self, id: CachedComputePipelineId) -> &mut Self {
        self.compute_pass.set_compute_pipeline(id);

        self
    }

    pub fn pass_node_builder(&mut self) -> &mut PassNodeBuilder<'a> {
        &mut self.pass_builder.pass_node_builder
    }

    pub fn end_pipeline_statistics_query(&mut self) -> &mut Self {
        self.compute_pass.end_pipeline_statistics_query();

        self
    }

    pub fn begin_pipeline_statistics_query(
        &mut self,
        query_set: &QuerySet,
        index: u32,
    ) -> &mut Self {
        self.compute_pass
            .begin_pipeline_statistics_query(query_set, index);

        self
    }

    pub fn write_timestamp(&mut self, query_set: &QuerySet, index: u32) -> &mut Self {
        self.compute_pass.write_timestamp(query_set, index);

        self
    }

    pub fn pop_debug_group(&mut self) -> &mut Self {
        self.compute_pass.pop_debug_group();

        self
    }

    pub fn push_debug_group(&mut self, label: &str) -> &mut Self {
        self.compute_pass.push_debug_group(label);

        self
    }

    pub fn insert_debug_marker(&mut self, label: &str) -> &mut Self {
        self.compute_pass.insert_debug_marker(label);

        self
    }

    pub fn set_raw_bind_group(
        &mut self,
        index: u32,
        bind_group: Option<&BindGroup>,
        offsets: &[u32],
    ) -> &mut Self {
        self.compute_pass
            .set_raw_bind_group(index, bind_group, offsets);

        self
    }

    pub fn set_bind_group<T>(&mut self, index: u32, bind_group: T, offsets: &[u32]) -> &mut Self
    where
        T: ResourceHandle<Drawing = BindGroupDrawing>,
    {
        let bind_group_ref = bind_group.make_resource_drawing(&mut self.pass_node_builder());

        self.compute_pass
            .set_bind_group(index, &bind_group_ref, offsets);
        self
    }
}
