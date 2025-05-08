use std::mem::take;

use tracing::warn;
use wgpu::QuerySet;

use crate::{
    frame_graph::{BindGroupDrawing, ComputePassCommandBuilder, PassNodeBuilder, ResourceHandle},
    render_resource::{BindGroup, CachedComputePipelineId},
};

use super::ComputePass;

pub struct ComputePassBuilder<'a> {
    compute_pass: ComputePass,
    pass_node_builder: PassNodeBuilder<'a>,
}

impl<'a> Drop for ComputePassBuilder<'a> {
    fn drop(&mut self) {
        let compute_pass = take(&mut self.compute_pass);

        if compute_pass.is_vaild() {
            self.pass_node_builder.set_pass(compute_pass);
        } else {
            warn!("render pass must is vaild");
        }
    }
}

impl<'a> ComputePassBuilder<'a> {
    pub fn new(pass_node_builder: PassNodeBuilder<'a>) -> Self {
        let compute_pass = ComputePass::default();

        Self {
            compute_pass,
            pass_node_builder,
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
        &mut self.pass_node_builder
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
        let bind_group_ref = bind_group.make_resource_drawing(&mut self.pass_node_builder);

        self.compute_pass
            .set_bind_group(index, &bind_group_ref, offsets);
        self
    }
}
