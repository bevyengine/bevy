use super::{
    pipelines::SolariGlobalIlluminationPipelineIds,
    view_resources::{SolariGlobalIlluminationBindGroups, SolariGlobalIlluminationViewResources},
    WORLD_CACHE_SIZE,
};
use crate::solari::scene::SolariSceneBindGroup;
use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{ComputePassDescriptor, PipelineCache},
    renderer::RenderContext,
    view::{ViewDepthTexture, ViewUniformOffset},
};

#[derive(Default)]
pub struct SolariGlobalIlluminationNode;

impl ViewNode for SolariGlobalIlluminationNode {
    type ViewQuery = (
        &'static SolariGlobalIlluminationPipelineIds,
        &'static SolariGlobalIlluminationBindGroups,
        &'static SolariGlobalIlluminationViewResources,
        &'static ViewPrepassTextures,
        &'static ViewDepthTexture,
        &'static ExtractedCamera,
        &'static ViewUniformOffset,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            pipeline_ids,
            bind_groups,
            view_resources,
            prepass_textures,
            depth_texture,
            camera,
            view_uniform_offset,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let (
            Some(pipeline_cache),
            Some(SolariSceneBindGroup(Some(scene_bind_group))),
            Some(viewport_size),
        ) = (
            world.get_resource::<PipelineCache>(),
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
            Some(screen_probes_trace_pipeline),
            Some(screen_probes_merge_cascades_pipeline),
            Some(screen_probes_filter_first_pass_pipeline),
            Some(screen_probes_filter_second_pass_pipeline),
            Some(screen_probes_interpolate_pipeline),
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
            pipeline_cache.get_compute_pipeline(pipeline_ids.screen_probes_trace),
            pipeline_cache.get_compute_pipeline(pipeline_ids.screen_probes_merge_cascades),
            pipeline_cache.get_compute_pipeline(pipeline_ids.screen_probes_filter_first_pass),
            pipeline_cache.get_compute_pipeline(pipeline_ids.screen_probes_filter_second_pass),
            pipeline_cache.get_compute_pipeline(pipeline_ids.screen_probes_interpolate),
            pipeline_cache.get_compute_pipeline(pipeline_ids.denoise_diffuse_temporal),
            pipeline_cache.get_compute_pipeline(pipeline_ids.denoise_diffuse_spatial),
        )
        else {
            return Ok(());
        };

        let width = (viewport_size.x + 7) / 8;
        let height = (viewport_size.y + 7) / 8;

        let command_encoder = render_context.command_encoder();
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

        solari_pass.set_pipeline(screen_probes_trace_pipeline);
        solari_pass.dispatch_workgroups(width, height, 4);

        solari_pass.set_pipeline(screen_probes_merge_cascades_pipeline);
        for cascade in (0..3u32).rev() {
            solari_pass.set_push_constants(0, &cascade.to_le_bytes());
            solari_pass.dispatch_workgroups(width, height, 1);
        }

        solari_pass.set_pipeline(screen_probes_filter_first_pass_pipeline);
        solari_pass.dispatch_workgroups(width, height, 1);

        solari_pass.set_pipeline(screen_probes_filter_second_pass_pipeline);
        solari_pass.dispatch_workgroups(width, height, 1);

        solari_pass.set_pipeline(screen_probes_interpolate_pipeline);
        solari_pass.dispatch_workgroups(width, height, 1);

        solari_pass.set_pipeline(denoise_diffuse_temporal_pipeline);
        solari_pass.dispatch_workgroups(width, height, 1);

        solari_pass.set_pipeline(denoise_diffuse_spatial_pipeline);
        solari_pass.dispatch_workgroups(width, height, 1);

        solari_pass.pop_debug_group();

        drop(solari_pass);

        // TODO: Should double buffer instead of copying
        command_encoder.copy_texture_to_texture(
            depth_texture.texture.as_image_copy(),
            view_resources.previous_depth_buffer.texture.as_image_copy(),
            prepass_textures.size,
        );
        command_encoder.copy_texture_to_texture(
            prepass_textures
                .normal
                .as_ref()
                .unwrap()
                .texture
                .as_image_copy(),
            view_resources
                .previous_normals_buffer
                .texture
                .as_image_copy(),
            prepass_textures.size,
        );

        Ok(())
    }
}
