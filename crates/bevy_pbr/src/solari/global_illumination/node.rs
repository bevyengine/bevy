use super::{
    pipelines::SolariGlobalIlluminationPipelineIds,
    view_resources::{SolariGlobalIlluminationBindGroups, SolariGlobalIlluminationViewResources},
    WORLD_CACHE_SIZE,
};
use crate::solari::scene::SolariSceneBindGroup;
use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{CommandEncoderDescriptor, ComputePassDescriptor, PipelineCache},
    renderer::{RenderContext, RenderQueue},
    view::ViewUniformOffset,
};

#[derive(Default)]
pub struct SolariGlobalIlluminationNode;

impl ViewNode for SolariGlobalIlluminationNode {
    type ViewQuery = (
        &'static SolariGlobalIlluminationPipelineIds,
        &'static SolariGlobalIlluminationBindGroups,
        &'static SolariGlobalIlluminationViewResources,
        &'static ExtractedCamera,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (pipeline_ids, bind_groups, view_resources, camera, view_uniform_offset): QueryItem<
            Self::ViewQuery,
        >,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let (
            Some(pipeline_cache),
            Some(render_queue),
            Some(SolariSceneBindGroup(Some(scene_bind_group))),
            Some(viewport_size),
        ) = (
            world.get_resource::<PipelineCache>(),
            world.get_resource::<RenderQueue>(),
            world.get_resource::<SolariSceneBindGroup>(),
            camera.physical_viewport_size,
        )
        else {
            return Ok(());
        };
        let (
            Some(decay_world_cache_pipeline),
            Some(compact_world_cache_single_block_pipeline),
            Some(compact_world_cache_blocks_pipeline),
            Some(compact_world_cache_write_active_cells_pipeline),
            Some(sample_for_world_cache_pipeline),
            Some(blend_new_world_cache_samples_pipeline),
            Some(update_screen_probes_pipeline),
            Some(filter_screen_probes_pipeline),
            Some(intepolate_screen_probes_pipeline),
            Some(denoise_diffuse_temporal_pipeline),
            Some(denoise_diffuse_spatial_pipeline),
        ) = (
            pipeline_cache.get_compute_pipeline(pipeline_ids.decay_world_cache),
            pipeline_cache.get_compute_pipeline(pipeline_ids.compact_world_cache_single_block),
            pipeline_cache.get_compute_pipeline(pipeline_ids.compact_world_cache_blocks),
            pipeline_cache
                .get_compute_pipeline(pipeline_ids.compact_world_cache_write_active_cells),
            pipeline_cache.get_compute_pipeline(pipeline_ids.sample_for_world_cache),
            pipeline_cache.get_compute_pipeline(pipeline_ids.blend_new_world_cache_samples),
            pipeline_cache.get_compute_pipeline(pipeline_ids.update_screen_probes),
            pipeline_cache.get_compute_pipeline(pipeline_ids.filter_screen_probes),
            pipeline_cache.get_compute_pipeline(pipeline_ids.interpolate_screen_probes),
            pipeline_cache.get_compute_pipeline(pipeline_ids.denoise_diffuse_temporal),
            pipeline_cache.get_compute_pipeline(pipeline_ids.denoise_diffuse_spatial),
        )
        else {
            return Ok(());
        };

        let width = (viewport_size.x + 7) / 8;
        let height = (viewport_size.y + 7) / 8;

        let render_device = render_context.render_device();
        let mut command_encoder = render_device.create_command_encoder(&CommandEncoderDescriptor {
            label: Some("solari_global_illumination_pass"),
        });
        let mut solari_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("solari_global_illumination_pass"),
            timestamp_writes: None,
        });

        solari_pass.push_debug_group("world_cache_update");

        solari_pass.set_bind_group(0, scene_bind_group, &[]);
        solari_pass.set_bind_group(
            1,
            &bind_groups.view_with_world_cache_dispatch_bind_group,
            &[view_uniform_offset.offset],
        );

        solari_pass.set_pipeline(decay_world_cache_pipeline);
        solari_pass.dispatch_workgroups((WORLD_CACHE_SIZE / 1024) as u32, 1, 1);

        solari_pass.set_pipeline(compact_world_cache_single_block_pipeline);
        solari_pass.dispatch_workgroups((WORLD_CACHE_SIZE / 1024) as u32, 1, 1);

        solari_pass.set_pipeline(compact_world_cache_blocks_pipeline);
        solari_pass.dispatch_workgroups(1, 1, 1);

        solari_pass.set_pipeline(compact_world_cache_write_active_cells_pipeline);
        solari_pass.dispatch_workgroups((WORLD_CACHE_SIZE / 1024) as u32, 1, 1);

        solari_pass.set_bind_group(
            1,
            &bind_groups.view_bind_group,
            &[view_uniform_offset.offset],
        );

        solari_pass.set_pipeline(sample_for_world_cache_pipeline);
        solari_pass.dispatch_workgroups_indirect(
            &view_resources.world_cache_active_cells_dispatch.buffer,
            0,
        );

        solari_pass.set_pipeline(blend_new_world_cache_samples_pipeline);
        solari_pass.dispatch_workgroups_indirect(
            &view_resources.world_cache_active_cells_dispatch.buffer,
            0,
        );

        solari_pass.pop_debug_group();

        solari_pass.push_debug_group("diffuse_global_illumination");

        solari_pass.set_pipeline(update_screen_probes_pipeline);
        solari_pass.dispatch_workgroups(width, height, 1);

        solari_pass.set_pipeline(filter_screen_probes_pipeline);
        solari_pass.dispatch_workgroups(width, height, 1);

        solari_pass.set_pipeline(intepolate_screen_probes_pipeline);
        solari_pass.dispatch_workgroups(width, height, 1);

        solari_pass.set_pipeline(denoise_diffuse_temporal_pipeline);
        solari_pass.dispatch_workgroups(width, height, 1);

        solari_pass.set_pipeline(denoise_diffuse_spatial_pipeline);
        solari_pass.dispatch_workgroups(width, height, 1);

        solari_pass.pop_debug_group();

        render_queue.submit([command_encoder.finish()]);

        Ok(())
    }
}
