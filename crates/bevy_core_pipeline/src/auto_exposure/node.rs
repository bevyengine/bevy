use super::{
    compensation_curve::GpuAutoExposureCompensationCurve,
    pipeline::{AutoExposurePipeline, ViewAutoExposurePipeline},
    AutoExposureBuffers, AutoExposureResources,
};
use bevy_ecs::{
    query::QueryState,
    system::lifetimeless::Read,
    world::{FromWorld, World},
};
use bevy_render::{
    frame_graph::{FrameGraph, FrameGraphTexture, GraphResourceNodeHandle, PassBuilder},
    globals::GlobalsBuffer,
    render_asset::RenderAssets,
    render_graph::*,
    render_resource::PipelineCache,
    texture::{FallbackImage, GpuImage},
    view::{ExtractedView, ViewTarget, ViewUniformOffset, ViewUniforms},
};

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct AutoExposure;

pub struct AutoExposureNode {
    query: QueryState<(
        Read<ViewUniformOffset>,
        Read<ViewTarget>,
        Read<ViewAutoExposurePipeline>,
        Read<ExtractedView>,
    )>,
}

impl FromWorld for AutoExposureNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for AutoExposureNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();
        let pipeline_cache = world.resource::<PipelineCache>();
        let pipeline = world.resource::<AutoExposurePipeline>();
        let resources = world.resource::<AutoExposureResources>();

        let view_uniforms_resource = world.resource::<ViewUniforms>();

        let globals_buffer = world.resource::<GlobalsBuffer>();
        let auto_exposure_buffers = world.resource::<AutoExposureBuffers>();

        let (
            Ok((view_uniform_offset, view_target, auto_exposure, view)),
            Some(auto_exposure_buffers),
        ) = (
            self.query.get_manual(world, view_entity),
            auto_exposure_buffers.buffers.get(&view_entity),
        )
        else {
            return Ok(());
        };

        let (Some(_), Some(_)) = (
            pipeline_cache.get_compute_pipeline(auto_exposure.histogram_pipeline),
            pipeline_cache.get_compute_pipeline(auto_exposure.mean_luminance_pipeline),
        ) else {
            return Ok(());
        };

        let source: GraphResourceNodeHandle<FrameGraphTexture> =
            frame_graph.get(view_target.get_main_texture_key())?;

        let fallback = world.resource::<FallbackImage>();
        let mask = world
            .resource::<RenderAssets<GpuImage>>()
            .get(&auto_exposure.metering_mask)
            .unwrap_or(&fallback.d2);

        let Some(compensation_curve) = world
            .resource::<RenderAssets<GpuAutoExposureCompensationCurve>>()
            .get(&auto_exposure.compensation_curve)
        else {
            return Ok(());
        };

        let mut pass_builder =
            PassBuilder::new(frame_graph.create_pass_node_bulder("auto_exposure_pass"));

        let compute_bind_group = pass_builder
            .create_bind_group_builder(None, pipeline.histogram_layout.clone())
            .push_bind_group_entry(&globals_buffer.buffer)
            .push_bind_group_entry(&auto_exposure_buffers.settings)
            .push_bind_group_entry(&source)
            .push_bind_group_entry(mask)
            .push_bind_group_entry(&compensation_curve.texture)
            .push_bind_group_entry(&compensation_curve.extents)
            .push_bind_group_entry(&resources.histogram)
            .push_bind_group_entry(&auto_exposure_buffers.state)
            .push_bind_group_entry(&view_uniforms_resource.uniforms)
            .build();

        let mut builder = pass_builder.create_compute_pass_builder();

        builder
            .set_bind_group(0, compute_bind_group, &[view_uniform_offset.offset])
            .set_compute_pipeline(auto_exposure.histogram_pipeline)
            .dispatch_workgroups(
                view.viewport.z.div_ceil(16),
                view.viewport.w.div_ceil(16),
                1,
            )
            .set_compute_pipeline(auto_exposure.mean_luminance_pipeline)
            .dispatch_workgroups(1, 1, 1);

        Ok(())
    }
}
