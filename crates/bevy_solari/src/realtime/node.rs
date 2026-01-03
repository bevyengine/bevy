use super::{
    prepare::{SolariLightingResources, LIGHT_TILE_BLOCKS, WORLD_CACHE_SIZE},
    SolariLighting,
};
use crate::scene::RaytracingSceneBindings;
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy_anti_alias::dlss::ViewDlssRayReconstructionTextures;
use bevy_asset::{load_embedded_asset, Handle};
use bevy_core_pipeline::prepass::{
    PreviousViewData, PreviousViewUniformOffset, PreviousViewUniforms, ViewPrepassTextures,
};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    query::QueryItem,
    world::{FromWorld, World},
};
use bevy_render::{
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        binding_types::{
            storage_buffer_sized, texture_2d, texture_depth_2d, texture_storage_2d, uniform_buffer,
        },
        BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
        CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor, LoadOp,
        PipelineCache, PushConstantRange, RenderPassDescriptor, ShaderStages, StorageTextureAccess,
        TextureFormat, TextureSampleType,
    },
    renderer::RenderContext,
    view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;

pub mod graph {
    use bevy_render::render_graph::RenderLabel;

    #[derive(Debug, Hash, PartialEq, Eq, Clone, RenderLabel)]
    pub struct SolariLightingNode;
}

pub struct SolariLightingNode {
    bind_group_layout: BindGroupLayoutDescriptor,
    bind_group_layout_world_cache_active_cells_dispatch: BindGroupLayoutDescriptor,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    bind_group_layout_resolve_dlss_rr_textures: BindGroupLayoutDescriptor,
    decay_world_cache_pipeline: CachedComputePipelineId,
    compact_world_cache_single_block_pipeline: CachedComputePipelineId,
    compact_world_cache_blocks_pipeline: CachedComputePipelineId,
    compact_world_cache_write_active_cells_pipeline: CachedComputePipelineId,
    sample_di_for_world_cache_pipeline: CachedComputePipelineId,
    sample_gi_for_world_cache_pipeline: CachedComputePipelineId,
    blend_new_world_cache_samples_pipeline: CachedComputePipelineId,
    presample_light_tiles_pipeline: CachedComputePipelineId,
    di_initial_and_temporal_pipeline: CachedComputePipelineId,
    di_spatial_and_shade_pipeline: CachedComputePipelineId,
    gi_initial_and_temporal_pipeline: CachedComputePipelineId,
    gi_spatial_and_shade_pipeline: CachedComputePipelineId,
    specular_gi_pipeline: CachedComputePipelineId,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    resolve_dlss_rr_textures_pipeline: CachedComputePipelineId,
}

impl ViewNode for SolariLightingNode {
    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
    type ViewQuery = (
        &'static SolariLighting,
        &'static SolariLightingResources,
        &'static ViewTarget,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
        &'static PreviousViewUniformOffset,
    );
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    type ViewQuery = (
        &'static SolariLighting,
        &'static SolariLightingResources,
        &'static ViewTarget,
        &'static ViewPrepassTextures,
        &'static ViewUniformOffset,
        &'static PreviousViewUniformOffset,
        Option<&'static ViewDlssRayReconstructionTextures>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))] (
            solari_lighting,
            solari_lighting_resources,
            view_target,
            view_prepass_textures,
            view_uniform_offset,
            previous_view_uniform_offset,
        ): QueryItem<Self::ViewQuery>,
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] (
            solari_lighting,
            solari_lighting_resources,
            view_target,
            view_prepass_textures,
            view_uniform_offset,
            previous_view_uniform_offset,
            view_dlss_rr_textures,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let scene_bindings = world.resource::<RaytracingSceneBindings>();
        let view_uniforms = world.resource::<ViewUniforms>();
        let previous_view_uniforms = world.resource::<PreviousViewUniforms>();
        let frame_count = world.resource::<FrameCount>();
        let (
            Some(decay_world_cache_pipeline),
            Some(compact_world_cache_single_block_pipeline),
            Some(compact_world_cache_blocks_pipeline),
            Some(compact_world_cache_write_active_cells_pipeline),
            Some(sample_di_for_world_cache_pipeline),
            Some(sample_gi_for_world_cache_pipeline),
            Some(blend_new_world_cache_samples_pipeline),
            Some(presample_light_tiles_pipeline),
            Some(di_initial_and_temporal_pipeline),
            Some(di_spatial_and_shade_pipeline),
            Some(gi_initial_and_temporal_pipeline),
            Some(gi_spatial_and_shade_pipeline),
            Some(specular_gi_pipeline),
            Some(scene_bindings),
            Some(gbuffer),
            Some(depth_buffer),
            Some(motion_vectors),
            Some(previous_gbuffer),
            Some(previous_depth_buffer),
            Some(view_uniforms),
            Some(previous_view_uniforms),
        ) = (
            pipeline_cache.get_compute_pipeline(self.decay_world_cache_pipeline),
            pipeline_cache.get_compute_pipeline(self.compact_world_cache_single_block_pipeline),
            pipeline_cache.get_compute_pipeline(self.compact_world_cache_blocks_pipeline),
            pipeline_cache
                .get_compute_pipeline(self.compact_world_cache_write_active_cells_pipeline),
            pipeline_cache.get_compute_pipeline(self.sample_di_for_world_cache_pipeline),
            pipeline_cache.get_compute_pipeline(self.sample_gi_for_world_cache_pipeline),
            pipeline_cache.get_compute_pipeline(self.blend_new_world_cache_samples_pipeline),
            pipeline_cache.get_compute_pipeline(self.presample_light_tiles_pipeline),
            pipeline_cache.get_compute_pipeline(self.di_initial_and_temporal_pipeline),
            pipeline_cache.get_compute_pipeline(self.di_spatial_and_shade_pipeline),
            pipeline_cache.get_compute_pipeline(self.gi_initial_and_temporal_pipeline),
            pipeline_cache.get_compute_pipeline(self.gi_spatial_and_shade_pipeline),
            pipeline_cache.get_compute_pipeline(self.specular_gi_pipeline),
            &scene_bindings.bind_group,
            view_prepass_textures.deferred_view(),
            view_prepass_textures.depth_view(),
            view_prepass_textures.motion_vectors_view(),
            view_prepass_textures.previous_deferred_view(),
            view_prepass_textures.previous_depth_view(),
            view_uniforms.uniforms.binding(),
            previous_view_uniforms.uniforms.binding(),
        )
        else {
            return Ok(());
        };
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        let Some(resolve_dlss_rr_textures_pipeline) =
            pipeline_cache.get_compute_pipeline(self.resolve_dlss_rr_textures_pipeline)
        else {
            return Ok(());
        };

        let view_target = view_target.get_unsampled_color_attachment();

        let s = solari_lighting_resources;
        let bind_group = render_context.render_device().create_bind_group(
            "solari_lighting_bind_group",
            &pipeline_cache.get_bind_group_layout(&self.bind_group_layout),
            &BindGroupEntries::sequential((
                view_target.view,
                s.light_tile_samples.as_entire_binding(),
                s.light_tile_resolved_samples.as_entire_binding(),
                &s.di_reservoirs_a.1,
                &s.di_reservoirs_b.1,
                s.gi_reservoirs_a.as_entire_binding(),
                s.gi_reservoirs_b.as_entire_binding(),
                gbuffer,
                depth_buffer,
                motion_vectors,
                previous_gbuffer,
                previous_depth_buffer,
                view_uniforms,
                previous_view_uniforms,
                s.world_cache_checksums.as_entire_binding(),
                s.world_cache_life.as_entire_binding(),
                s.world_cache_radiance.as_entire_binding(),
                s.world_cache_geometry_data.as_entire_binding(),
                s.world_cache_luminance_deltas.as_entire_binding(),
                s.world_cache_active_cells_new_radiance.as_entire_binding(),
                s.world_cache_a.as_entire_binding(),
                s.world_cache_b.as_entire_binding(),
                s.world_cache_active_cell_indices.as_entire_binding(),
                s.world_cache_active_cells_count.as_entire_binding(),
            )),
        );
        let bind_group_world_cache_active_cells_dispatch =
            render_context.render_device().create_bind_group(
                "solari_lighting_bind_group_world_cache_active_cells_dispatch",
                &pipeline_cache.get_bind_group_layout(
                    &self.bind_group_layout_world_cache_active_cells_dispatch,
                ),
                &BindGroupEntries::single(s.world_cache_active_cells_dispatch.as_entire_binding()),
            );
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        let bind_group_resolve_dlss_rr_textures = view_dlss_rr_textures.map(|d| {
            render_context.render_device().create_bind_group(
                "solari_lighting_bind_group_resolve_dlss_rr_textures",
                &pipeline_cache
                    .get_bind_group_layout(&self.bind_group_layout_resolve_dlss_rr_textures),
                &BindGroupEntries::sequential((
                    &d.diffuse_albedo.default_view,
                    &d.specular_albedo.default_view,
                    &d.normal_roughness.default_view,
                    &d.specular_motion_vectors.default_view,
                )),
            )
        });

        // Choice of number here is arbitrary
        let frame_index = frame_count.0.wrapping_mul(5782582);

        let diagnostics = render_context.diagnostic_recorder();
        let command_encoder = render_context.command_encoder();

        // Clear the view target if we're the first node to write to it
        if matches!(view_target.ops.load, LoadOp::Clear(_)) {
            command_encoder.begin_render_pass(&RenderPassDescriptor {
                label: Some("solari_lighting_clear"),
                color_attachments: &[Some(view_target)],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
        }

        let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("solari_lighting"),
            timestamp_writes: None,
        });

        let dx = solari_lighting_resources.view_size.x.div_ceil(8);
        let dy = solari_lighting_resources.view_size.y.div_ceil(8);

        pass.set_bind_group(0, scene_bindings, &[]);
        pass.set_bind_group(
            1,
            &bind_group,
            &[
                view_uniform_offset.offset,
                previous_view_uniform_offset.offset,
            ],
        );

        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        if let Some(bind_group_resolve_dlss_rr_textures) = bind_group_resolve_dlss_rr_textures {
            pass.set_bind_group(2, &bind_group_resolve_dlss_rr_textures, &[]);
            pass.set_pipeline(resolve_dlss_rr_textures_pipeline);
            pass.dispatch_workgroups(dx, dy, 1);
        }

        let d = diagnostics.time_span(&mut pass, "solari_lighting/presample_light_tiles");
        pass.set_pipeline(presample_light_tiles_pipeline);
        pass.set_push_constants(
            0,
            bytemuck::cast_slice(&[frame_index, solari_lighting.reset as u32]),
        );
        pass.dispatch_workgroups(LIGHT_TILE_BLOCKS as u32, 1, 1);
        d.end(&mut pass);

        let d = diagnostics.time_span(&mut pass, "solari_lighting/world_cache");

        pass.set_bind_group(2, &bind_group_world_cache_active_cells_dispatch, &[]);

        pass.set_pipeline(decay_world_cache_pipeline);
        pass.dispatch_workgroups((WORLD_CACHE_SIZE / 1024) as u32, 1, 1);

        pass.set_pipeline(compact_world_cache_single_block_pipeline);
        pass.dispatch_workgroups((WORLD_CACHE_SIZE / 1024) as u32, 1, 1);

        pass.set_pipeline(compact_world_cache_blocks_pipeline);
        pass.dispatch_workgroups(1, 1, 1);

        pass.set_pipeline(compact_world_cache_write_active_cells_pipeline);
        pass.dispatch_workgroups((WORLD_CACHE_SIZE / 1024) as u32, 1, 1);

        pass.set_bind_group(2, None, &[]);

        pass.set_pipeline(sample_di_for_world_cache_pipeline);
        pass.set_push_constants(
            0,
            bytemuck::cast_slice(&[frame_index, solari_lighting.reset as u32]),
        );
        pass.dispatch_workgroups_indirect(
            &solari_lighting_resources.world_cache_active_cells_dispatch,
            0,
        );

        pass.set_pipeline(sample_gi_for_world_cache_pipeline);
        pass.set_push_constants(
            0,
            bytemuck::cast_slice(&[frame_index, solari_lighting.reset as u32]),
        );
        pass.dispatch_workgroups_indirect(
            &solari_lighting_resources.world_cache_active_cells_dispatch,
            0,
        );

        pass.set_pipeline(blend_new_world_cache_samples_pipeline);
        pass.dispatch_workgroups_indirect(
            &solari_lighting_resources.world_cache_active_cells_dispatch,
            0,
        );

        d.end(&mut pass);

        let d = diagnostics.time_span(&mut pass, "solari_lighting/direct_lighting");

        pass.set_pipeline(di_initial_and_temporal_pipeline);
        pass.set_push_constants(
            0,
            bytemuck::cast_slice(&[frame_index, solari_lighting.reset as u32]),
        );
        pass.dispatch_workgroups(dx, dy, 1);

        pass.set_pipeline(di_spatial_and_shade_pipeline);
        pass.set_push_constants(
            0,
            bytemuck::cast_slice(&[frame_index, solari_lighting.reset as u32]),
        );
        pass.dispatch_workgroups(dx, dy, 1);

        d.end(&mut pass);

        let d = diagnostics.time_span(&mut pass, "solari_lighting/diffuse_indirect_lighting");

        pass.set_pipeline(gi_initial_and_temporal_pipeline);
        pass.set_push_constants(
            0,
            bytemuck::cast_slice(&[frame_index, solari_lighting.reset as u32]),
        );
        pass.dispatch_workgroups(dx, dy, 1);

        pass.set_pipeline(gi_spatial_and_shade_pipeline);
        pass.set_push_constants(
            0,
            bytemuck::cast_slice(&[frame_index, solari_lighting.reset as u32]),
        );
        pass.dispatch_workgroups(dx, dy, 1);

        d.end(&mut pass);

        let d = diagnostics.time_span(&mut pass, "solari_lighting/specular_indirect_lighting");
        pass.set_pipeline(specular_gi_pipeline);
        pass.set_push_constants(
            0,
            bytemuck::cast_slice(&[frame_index, solari_lighting.reset as u32]),
        );
        pass.dispatch_workgroups(dx, dy, 1);
        d.end(&mut pass);

        drop(pass);

        diagnostics.record_u32(
            render_context.command_encoder(),
            &s.world_cache_active_cells_count.slice(..),
            "solari_lighting/world_cache_active_cells_count",
        );

        Ok(())
    }
}

impl FromWorld for SolariLightingNode {
    fn from_world(world: &mut World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let scene_bindings = world.resource::<RaytracingSceneBindings>();

        let bind_group_layout = BindGroupLayoutDescriptor::new(
            "solari_lighting_bind_group_layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_storage_2d(
                        ViewTarget::TEXTURE_FORMAT_HDR,
                        StorageTextureAccess::ReadWrite,
                    ),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    texture_storage_2d(TextureFormat::Rgba32Uint, StorageTextureAccess::ReadWrite),
                    texture_storage_2d(TextureFormat::Rgba32Uint, StorageTextureAccess::ReadWrite),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    texture_2d(TextureSampleType::Uint),
                    texture_depth_2d(),
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    texture_2d(TextureSampleType::Uint),
                    texture_depth_2d(),
                    uniform_buffer::<ViewUniform>(true),
                    uniform_buffer::<PreviousViewData>(true),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                    storage_buffer_sized(false, None),
                ),
            ),
        );

        let bind_group_layout_world_cache_active_cells_dispatch = BindGroupLayoutDescriptor::new(
            "solari_lighting_bind_group_layout_world_cache_active_cells_dispatch",
            &BindGroupLayoutEntries::single(
                ShaderStages::COMPUTE,
                storage_buffer_sized(false, None),
            ),
        );

        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        let bind_group_layout_resolve_dlss_rr_textures = BindGroupLayoutDescriptor::new(
            "solari_lighting_bind_group_layout_resolve_dlss_rr_textures",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::Rgba8Unorm, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                    texture_storage_2d(TextureFormat::Rg16Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        );

        let create_pipeline = |label: &'static str,
                               entry_point: &'static str,
                               shader: Handle<Shader>,
                               extra_bind_group_layout: Option<&BindGroupLayoutDescriptor>,
                               extra_shader_defs: Vec<ShaderDefVal>| {
            let mut layout = vec![
                scene_bindings.bind_group_layout.clone(),
                bind_group_layout.clone(),
            ];
            if let Some(extra_bind_group_layout) = extra_bind_group_layout {
                layout.push(extra_bind_group_layout.clone());
            }

            let mut shader_defs = vec![ShaderDefVal::UInt(
                "WORLD_CACHE_SIZE".into(),
                WORLD_CACHE_SIZE as u32,
            )];
            shader_defs.extend_from_slice(&extra_shader_defs);

            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some(label.into()),
                layout,
                push_constant_ranges: vec![PushConstantRange {
                    stages: ShaderStages::COMPUTE,
                    range: 0..8,
                }],
                shader,
                shader_defs,
                entry_point: Some(entry_point.into()),
                ..default()
            })
        };

        Self {
            bind_group_layout: bind_group_layout.clone(),
            bind_group_layout_world_cache_active_cells_dispatch:
                bind_group_layout_world_cache_active_cells_dispatch.clone(),
            #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
            bind_group_layout_resolve_dlss_rr_textures: bind_group_layout_resolve_dlss_rr_textures
                .clone(),
            decay_world_cache_pipeline: create_pipeline(
                "solari_lighting_decay_world_cache_pipeline",
                "decay_world_cache",
                load_embedded_asset!(world, "world_cache_compact.wgsl"),
                Some(&bind_group_layout_world_cache_active_cells_dispatch),
                vec!["WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER".into()],
            ),
            compact_world_cache_single_block_pipeline: create_pipeline(
                "solari_lighting_compact_world_cache_single_block_pipeline",
                "compact_world_cache_single_block",
                load_embedded_asset!(world, "world_cache_compact.wgsl"),
                Some(&bind_group_layout_world_cache_active_cells_dispatch),
                vec!["WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER".into()],
            ),
            compact_world_cache_blocks_pipeline: create_pipeline(
                "solari_lighting_compact_world_cache_blocks_pipeline",
                "compact_world_cache_blocks",
                load_embedded_asset!(world, "world_cache_compact.wgsl"),
                Some(&bind_group_layout_world_cache_active_cells_dispatch),
                vec![],
            ),
            compact_world_cache_write_active_cells_pipeline: create_pipeline(
                "solari_lighting_compact_world_cache_write_active_cells_pipeline",
                "compact_world_cache_write_active_cells",
                load_embedded_asset!(world, "world_cache_compact.wgsl"),
                Some(&bind_group_layout_world_cache_active_cells_dispatch),
                vec!["WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER".into()],
            ),
            sample_di_for_world_cache_pipeline: create_pipeline(
                "solari_lighting_sample_di_for_world_cache_pipeline",
                "sample_di",
                load_embedded_asset!(world, "world_cache_update.wgsl"),
                None,
                vec![],
            ),
            sample_gi_for_world_cache_pipeline: create_pipeline(
                "solari_lighting_sample_gi_for_world_cache_pipeline",
                "sample_gi",
                load_embedded_asset!(world, "world_cache_update.wgsl"),
                None,
                vec!["WORLD_CACHE_QUERY_ATOMIC_MAX_LIFETIME".into()],
            ),
            blend_new_world_cache_samples_pipeline: create_pipeline(
                "solari_lighting_blend_new_world_cache_samples_pipeline",
                "blend_new_samples",
                load_embedded_asset!(world, "world_cache_update.wgsl"),
                None,
                vec![],
            ),
            presample_light_tiles_pipeline: create_pipeline(
                "solari_lighting_presample_light_tiles_pipeline",
                "presample_light_tiles",
                load_embedded_asset!(world, "presample_light_tiles.wgsl"),
                None,
                vec![],
            ),
            di_initial_and_temporal_pipeline: create_pipeline(
                "solari_lighting_di_initial_and_temporal_pipeline",
                "initial_and_temporal",
                load_embedded_asset!(world, "restir_di.wgsl"),
                None,
                vec![],
            ),
            di_spatial_and_shade_pipeline: create_pipeline(
                "solari_lighting_di_spatial_and_shade_pipeline",
                "spatial_and_shade",
                load_embedded_asset!(world, "restir_di.wgsl"),
                None,
                vec![],
            ),
            gi_initial_and_temporal_pipeline: create_pipeline(
                "solari_lighting_gi_initial_and_temporal_pipeline",
                "initial_and_temporal",
                load_embedded_asset!(world, "restir_gi.wgsl"),
                None,
                vec!["WORLD_CACHE_FIRST_BOUNCE_LIGHT_LEAK_PREVENTION".into()],
            ),
            gi_spatial_and_shade_pipeline: create_pipeline(
                "solari_lighting_gi_spatial_and_shade_pipeline",
                "spatial_and_shade",
                load_embedded_asset!(world, "restir_gi.wgsl"),
                None,
                vec![],
            ),
            specular_gi_pipeline: create_pipeline(
                "solari_lighting_specular_gi_pipeline",
                "specular_gi",
                load_embedded_asset!(world, "specular_gi.wgsl"),
                None,
                vec![],
            ),
            #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
            resolve_dlss_rr_textures_pipeline: create_pipeline(
                "solari_lighting_resolve_dlss_rr_textures_pipeline",
                "resolve_dlss_rr_textures",
                load_embedded_asset!(world, "resolve_dlss_rr_textures.wgsl"),
                Some(&bind_group_layout_resolve_dlss_rr_textures),
                vec![],
            ),
        }
    }
}
