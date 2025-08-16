use super::{
    prepare::{SolariLightingResources, LIGHT_TILE_BLOCKS, WORLD_CACHE_SIZE},
    SolariLighting,
};
use crate::scene::RaytracingSceneBindings;
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy_anti_aliasing::dlss::ViewDlssRayReconstructionTextures;
use bevy_asset::{load_embedded_asset, Handle};
use bevy_core_pipeline::prepass::{
    PreviousViewData, PreviousViewUniformOffset, PreviousViewUniforms, ViewPrepassTextures,
};
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    query::QueryItem,
    world::{FromWorld, World},
};
use bevy_image::ToExtents;
use bevy_render::{
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{
        binding_types::{
            storage_buffer_sized, texture_2d, texture_depth_2d, texture_storage_2d, uniform_buffer,
        },
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedComputePipelineId,
        ComputePassDescriptor, ComputePipelineDescriptor, PipelineCache, PushConstantRange,
        ShaderStages, StorageTextureAccess, TextureFormat, TextureSampleType,
    },
    renderer::{RenderContext, RenderDevice},
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
    bind_group_layout: BindGroupLayout,
    bind_group_layout_world_cache_active_cells_dispatch: BindGroupLayout,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    bind_group_layout_resolve_dlss_rr_textures: BindGroupLayout,
    decay_world_cache_pipeline: CachedComputePipelineId,
    compact_world_cache_single_block_pipeline: CachedComputePipelineId,
    compact_world_cache_blocks_pipeline: CachedComputePipelineId,
    compact_world_cache_write_active_cells_pipeline: CachedComputePipelineId,
    sample_for_world_cache_pipeline: CachedComputePipelineId,
    blend_new_world_cache_samples_pipeline: CachedComputePipelineId,
    presample_light_tiles_pipeline: CachedComputePipelineId,
    di_initial_and_temporal_pipeline: CachedComputePipelineId,
    di_spatial_and_shade_pipeline: CachedComputePipelineId,
    gi_initial_and_temporal_pipeline: CachedComputePipelineId,
    gi_spatial_and_shade_pipeline: CachedComputePipelineId,
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
            Some(sample_for_world_cache_pipeline),
            Some(blend_new_world_cache_samples_pipeline),
            Some(presample_light_tiles_pipeline),
            Some(di_initial_and_temporal_pipeline),
            Some(di_spatial_and_shade_pipeline),
            Some(gi_initial_and_temporal_pipeline),
            Some(gi_spatial_and_shade_pipeline),
            Some(scene_bindings),
            Some(gbuffer),
            Some(depth_buffer),
            Some(motion_vectors),
            Some(view_uniforms),
            Some(previous_view_uniforms),
        ) = (
            pipeline_cache.get_compute_pipeline(self.decay_world_cache_pipeline),
            pipeline_cache.get_compute_pipeline(self.compact_world_cache_single_block_pipeline),
            pipeline_cache.get_compute_pipeline(self.compact_world_cache_blocks_pipeline),
            pipeline_cache
                .get_compute_pipeline(self.compact_world_cache_write_active_cells_pipeline),
            pipeline_cache.get_compute_pipeline(self.sample_for_world_cache_pipeline),
            pipeline_cache.get_compute_pipeline(self.blend_new_world_cache_samples_pipeline),
            pipeline_cache.get_compute_pipeline(self.presample_light_tiles_pipeline),
            pipeline_cache.get_compute_pipeline(self.di_initial_and_temporal_pipeline),
            pipeline_cache.get_compute_pipeline(self.di_spatial_and_shade_pipeline),
            pipeline_cache.get_compute_pipeline(self.gi_initial_and_temporal_pipeline),
            pipeline_cache.get_compute_pipeline(self.gi_spatial_and_shade_pipeline),
            &scene_bindings.bind_group,
            view_prepass_textures.deferred_view(),
            view_prepass_textures.depth_view(),
            view_prepass_textures.motion_vectors_view(),
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

        let s = solari_lighting_resources;
        let bind_group = render_context.render_device().create_bind_group(
            "solari_lighting_bind_group",
            &self.bind_group_layout,
            &BindGroupEntries::sequential((
                view_target.get_unsampled_color_attachment().view,
                s.light_tile_samples.as_entire_binding(),
                s.light_tile_resolved_samples.as_entire_binding(),
                &s.di_reservoirs_a.1,
                &s.di_reservoirs_b.1,
                s.gi_reservoirs_a.as_entire_binding(),
                s.gi_reservoirs_b.as_entire_binding(),
                gbuffer,
                depth_buffer,
                motion_vectors,
                &s.previous_gbuffer.1,
                &s.previous_depth.1,
                view_uniforms,
                previous_view_uniforms,
                s.world_cache_checksums.as_entire_binding(),
                s.world_cache_life.as_entire_binding(),
                s.world_cache_radiance.as_entire_binding(),
                s.world_cache_geometry_data.as_entire_binding(),
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
                &self.bind_group_layout_world_cache_active_cells_dispatch,
                &BindGroupEntries::single(s.world_cache_active_cells_dispatch.as_entire_binding()),
            );
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        let bind_group_resolve_dlss_rr_textures = view_dlss_rr_textures.map(|d| {
            render_context.render_device().create_bind_group(
                "solari_lighting_bind_group_resolve_dlss_rr_textures",
                &self.bind_group_layout_resolve_dlss_rr_textures,
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

        let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("solari_lighting"),
            timestamp_writes: None,
        });
        let pass_span = diagnostics.pass_span(&mut pass, "solari_lighting");

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

        pass.set_pipeline(sample_for_world_cache_pipeline);
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

        pass.set_pipeline(presample_light_tiles_pipeline);
        pass.set_push_constants(
            0,
            bytemuck::cast_slice(&[frame_index, solari_lighting.reset as u32]),
        );
        pass.dispatch_workgroups(LIGHT_TILE_BLOCKS as u32, 1, 1);

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

        pass_span.end(&mut pass);
        drop(pass);

        // TODO: Remove these copies, and double buffer instead
        command_encoder.copy_texture_to_texture(
            view_prepass_textures
                .deferred
                .clone()
                .unwrap()
                .texture
                .texture
                .as_image_copy(),
            solari_lighting_resources.previous_gbuffer.0.as_image_copy(),
            solari_lighting_resources.view_size.to_extents(),
        );
        command_encoder.copy_texture_to_texture(
            view_prepass_textures
                .depth
                .clone()
                .unwrap()
                .texture
                .texture
                .as_image_copy(),
            solari_lighting_resources.previous_depth.0.as_image_copy(),
            solari_lighting_resources.view_size.to_extents(),
        );

        Ok(())
    }
}

impl FromWorld for SolariLightingNode {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let scene_bindings = world.resource::<RaytracingSceneBindings>();

        let bind_group_layout = render_device.create_bind_group_layout(
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
                ),
            ),
        );

        let bind_group_layout_world_cache_active_cells_dispatch = render_device
            .create_bind_group_layout(
                "solari_lighting_bind_group_layout_world_cache_active_cells_dispatch",
                &BindGroupLayoutEntries::single(
                    ShaderStages::COMPUTE,
                    storage_buffer_sized(false, None),
                ),
            );

        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        let bind_group_layout_resolve_dlss_rr_textures = render_device.create_bind_group_layout(
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
                               extra_bind_group_layout: Option<&BindGroupLayout>,
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
            sample_for_world_cache_pipeline: create_pipeline(
                "solari_lighting_sample_for_world_cache_pipeline",
                "sample_radiance",
                load_embedded_asset!(world, "world_cache_update.wgsl"),
                None,
                vec![],
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
                vec![],
            ),
            gi_spatial_and_shade_pipeline: create_pipeline(
                "solari_lighting_gi_spatial_and_shade_pipeline",
                "spatial_and_shade",
                load_embedded_asset!(world, "restir_gi.wgsl"),
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
