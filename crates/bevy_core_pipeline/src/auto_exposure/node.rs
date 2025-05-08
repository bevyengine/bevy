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
    frame_graph::{
        BindGroupDrawing, BindGroupEntryRefs, BindingResourceHandleHelper, ComputePassBuilder,
        FrameGraph, FrameGraphTexture, ResourceRead, ResourceRef,
    },
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

        let mut pass_node_builder = frame_graph.create_pass_node_bulder("auto_exposure_pass");

        let globals_buffer = globals_buffer
            .buffer
            .make_binding_resource_ref(&mut pass_node_builder);

        let auto_exposure_buffer = auto_exposure_buffers
            .settings
            .make_binding_resource_ref(&mut pass_node_builder);

        let source: ResourceRef<FrameGraphTexture, ResourceRead> =
            pass_node_builder.read_from_board(view_target.get_main_texture_key())?;

        let fallback = world.resource::<FallbackImage>();
        let mask = world
            .resource::<RenderAssets<GpuImage>>()
            .get(&auto_exposure.metering_mask)
            .unwrap_or(&fallback.d2);

        let mask = mask.make_binding_resource_ref(&mut pass_node_builder);

        let Some(compensation_curve) = world
            .resource::<RenderAssets<GpuAutoExposureCompensationCurve>>()
            .get(&auto_exposure.compensation_curve)
        else {
            return Ok(());
        };

        let compensation_curve_texture = compensation_curve
            .texture
            .make_binding_resource_ref(&mut pass_node_builder);

        let compensation_curve_extents = compensation_curve
            .extents
            .make_binding_resource_ref(&mut pass_node_builder);

        let resources_histogram = resources
            .histogram
            .make_binding_resource_ref(&mut pass_node_builder);

        let auto_exposure_buffers_state = auto_exposure_buffers
            .state
            .make_binding_resource_ref(&mut pass_node_builder);

        let view_uniforms_buffer = view_uniforms_resource
            .uniforms
            .make_binding_resource_ref(&mut pass_node_builder);

        let compute_bind_group = BindGroupDrawing {
            label: None,
            layout: pipeline.histogram_layout.clone(),
            entries: BindGroupEntryRefs::sequential((
                &globals_buffer,
                &auto_exposure_buffer,
                &source,
                &mask,
                &compensation_curve_texture,
                &compensation_curve_extents,
                &resources_histogram,
                &auto_exposure_buffers_state,
                &view_uniforms_buffer,
            ))
            .to_vec(),
        };

        let mut builder = ComputePassBuilder::new(pass_node_builder);

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
