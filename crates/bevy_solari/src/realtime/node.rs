use super::prepare::{
    SolariLightingResources, LIGHT_TILE_BLOCKS, WORLD_CACHE_ACTIVE_CELLS_COUNT_OFFSET,
    WORLD_CACHE_SIZE,
};
use crate::scene::RaytracingSceneBindings;
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy_anti_alias::dlss::ViewDlssRayReconstructionTextures;
use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_core_pipeline::prepass::{
    PreviousViewData, PreviousViewUniformOffset, PreviousViewUniforms, ViewPrepassTextures,
};
use bevy_ecs::{prelude::*, resource::Resource, system::Commands};
use bevy_render::{
    diagnostic::RecordDiagnostics as _,
    render_resource::{
        binding_types::{
            storage_buffer_sized, texture_2d, texture_depth_2d, texture_storage_2d, uniform_buffer,
            uniform_buffer_sized,
        },
        BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
        CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor, LoadOp,
        PipelineCache, RenderPassDescriptor, ShaderStages, StorageTextureAccess, TextureFormat,
        TextureSampleType,
    },
    renderer::{RenderContext, RenderDevice, ViewQuery},
    view::{ViewTarget, ViewUniform, ViewUniformOffset, ViewUniforms},
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;

/// Resource holding the Solari lighting pipeline configuration.
#[derive(Resource)]
pub struct SolariLightingPipelines {
    bind_group_layout: BindGroupLayoutDescriptor,
    bind_group_layout_restir: BindGroupLayoutDescriptor,
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
    restir: RestirPipelines,
    no_restir: NoRestirPipelines,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    resolve_dlss_rr_textures_pipeline: CachedComputePipelineId,
}

struct RestirPipelines {
    initial_and_temporal: CachedComputePipelineId,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    initial_and_temporal_with_psr: CachedComputePipelineId,
    spatial_and_shade: CachedComputePipelineId,
}

struct NoRestirPipelines {
    initial_and_shade: CachedComputePipelineId,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    initial_and_shade_with_psr: CachedComputePipelineId,
}

#[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
type SolariLightingViewQuery = (
    &'static SolariLightingResources,
    &'static ViewTarget,
    &'static ViewPrepassTextures,
    &'static ViewUniformOffset,
    &'static PreviousViewUniformOffset,
);

#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
type SolariLightingViewQuery = (
    &'static SolariLightingResources,
    &'static ViewTarget,
    &'static ViewPrepassTextures,
    &'static ViewUniformOffset,
    &'static PreviousViewUniformOffset,
    Option<&'static ViewDlssRayReconstructionTextures>,
);

pub fn solari_lighting(
    view: ViewQuery<SolariLightingViewQuery>,
    solari_pipelines: Option<Res<SolariLightingPipelines>>,
    pipeline_cache: Res<PipelineCache>,
    scene_bindings: Res<RaytracingSceneBindings>,
    view_uniforms: Res<ViewUniforms>,
    previous_view_uniforms: Res<PreviousViewUniforms>,
    render_device: Res<RenderDevice>,
    mut ctx: RenderContext,
) {
    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
    let (
        solari_lighting_resources,
        view_target,
        view_prepass_textures,
        view_uniform_offset,
        previous_view_uniform_offset,
    ) = view.into_inner();

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let (
        solari_lighting_resources,
        view_target,
        view_prepass_textures,
        view_uniform_offset,
        previous_view_uniform_offset,
        view_dlss_rr_textures,
    ) = view.into_inner();

    let Some(pipelines) = solari_pipelines else {
        return;
    };

    let restir = solari_lighting_resources.reservoirs.as_ref().zip(
        view_prepass_textures
            .previous_deferred_view()
            .zip(view_prepass_textures.previous_depth_only_view()),
    );

    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
    let (initial_pipeline_id, spatial_pipeline_id) = if restir.is_some() {
        (
            pipelines.restir.initial_and_temporal,
            Some(pipelines.restir.spatial_and_shade),
        )
    } else {
        (pipelines.no_restir.initial_and_shade, None)
    };

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let (initial_pipeline_id, spatial_pipeline_id) =
        match (restir.is_some(), view_dlss_rr_textures.is_some()) {
            (true, true) => (
                pipelines.restir.initial_and_temporal_with_psr,
                Some(pipelines.restir.spatial_and_shade),
            ),
            (true, false) => (
                pipelines.restir.initial_and_temporal,
                Some(pipelines.restir.spatial_and_shade),
            ),
            (false, true) => (pipelines.no_restir.initial_and_shade_with_psr, None),
            (false, false) => (pipelines.no_restir.initial_and_shade, None),
        };

    let (
        Some(decay_world_cache_pipeline),
        Some(compact_world_cache_single_block_pipeline),
        Some(compact_world_cache_blocks_pipeline),
        Some(compact_world_cache_write_active_cells_pipeline),
        Some(sample_di_for_world_cache_pipeline),
        Some(sample_gi_for_world_cache_pipeline),
        Some(blend_new_world_cache_samples_pipeline),
        Some(presample_light_tiles_pipeline),
        Some(initial_pipeline),
        Some(scene_bind_group),
        Some(gbuffer),
        Some(depth_buffer),
        Some(motion_vectors),
        Some(view_uniforms_binding),
        Some(previous_view_uniforms_binding),
    ) = (
        pipeline_cache.get_compute_pipeline(pipelines.decay_world_cache_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.compact_world_cache_single_block_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.compact_world_cache_blocks_pipeline),
        pipeline_cache
            .get_compute_pipeline(pipelines.compact_world_cache_write_active_cells_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.sample_di_for_world_cache_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.sample_gi_for_world_cache_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.blend_new_world_cache_samples_pipeline),
        pipeline_cache.get_compute_pipeline(pipelines.presample_light_tiles_pipeline),
        pipeline_cache.get_compute_pipeline(initial_pipeline_id),
        &scene_bindings.bind_group,
        view_prepass_textures.deferred_view(),
        view_prepass_textures.depth_only_view(),
        view_prepass_textures.motion_vectors_view(),
        view_uniforms.uniforms.binding(),
        previous_view_uniforms.uniforms.binding(),
    )
    else {
        return;
    };

    let spatial_and_shade_pipeline = match spatial_pipeline_id {
        Some(id) => match pipeline_cache.get_compute_pipeline(id) {
            None => return,
            pipeline => pipeline,
        },
        None => None,
    };

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let Some(resolve_dlss_rr_textures_pipeline) =
        pipeline_cache.get_compute_pipeline(pipelines.resolve_dlss_rr_textures_pipeline)
    else {
        return;
    };

    let view_target_attachment = view_target.get_unsampled_color_attachment();

    let s = solari_lighting_resources;
    let bind_group = render_device.create_bind_group(
        "solari_lighting_bind_group",
        &pipeline_cache.get_bind_group_layout(&pipelines.bind_group_layout),
        &BindGroupEntries::sequential((
            view_target_attachment.view,
            s.light_tile_samples.as_entire_binding(),
            s.light_tile_resolved_samples.as_entire_binding(),
            gbuffer,
            depth_buffer,
            motion_vectors,
            view_uniforms_binding.clone(),
            previous_view_uniforms_binding.clone(),
            s.world_cache.as_entire_binding(),
            s.constants.as_entire_binding(),
        )),
    );

    let bind_group_restir =
        restir.map(|(reservoirs, (previous_gbuffer, previous_depth_buffer))| {
            render_device.create_bind_group(
                "solari_lighting_bind_group_restir",
                &pipeline_cache.get_bind_group_layout(&pipelines.bind_group_layout_restir),
                &BindGroupEntries::sequential((
                    view_target_attachment.view,
                    s.light_tile_samples.as_entire_binding(),
                    s.light_tile_resolved_samples.as_entire_binding(),
                    gbuffer,
                    depth_buffer,
                    motion_vectors,
                    view_uniforms_binding,
                    previous_view_uniforms_binding,
                    s.world_cache.as_entire_binding(),
                    s.constants.as_entire_binding(),
                    previous_gbuffer,
                    previous_depth_buffer,
                    reservoirs.a.as_entire_binding(),
                    reservoirs.b.as_entire_binding(),
                )),
            )
        });

    let bind_group_world_cache_active_cells_dispatch = render_device.create_bind_group(
        "solari_lighting_bind_group_world_cache_active_cells_dispatch",
        &pipeline_cache
            .get_bind_group_layout(&pipelines.bind_group_layout_world_cache_active_cells_dispatch),
        &BindGroupEntries::single(s.world_cache_active_cells_dispatch.as_entire_binding()),
    );

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    let bind_group_resolve_dlss_rr_textures = view_dlss_rr_textures.map(|d| {
        render_device.create_bind_group(
            "solari_lighting_bind_group_resolve_dlss_rr_textures",
            &pipeline_cache
                .get_bind_group_layout(&pipelines.bind_group_layout_resolve_dlss_rr_textures),
            &BindGroupEntries::sequential((
                &d.diffuse_albedo.default_view,
                &d.specular_albedo.default_view,
                &d.normal_roughness.default_view,
                &d.depth.default_view,
                &d.specular_motion_vectors.default_view,
            )),
        )
    });

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let command_encoder = ctx.command_encoder();

    // Clear the view target if we're the first node to write to it
    if matches!(view_target_attachment.ops.load, LoadOp::Clear(_)) {
        command_encoder.begin_render_pass(&RenderPassDescriptor {
            label: Some("solari_lighting_clear"),
            color_attachments: &[Some(view_target_attachment)],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
    }

    let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("solari_lighting"),
        timestamp_writes: None,
    });

    let dx = solari_lighting_resources.view_size.x.div_ceil(8);
    let dy = solari_lighting_resources.view_size.y.div_ceil(8);

    pass.set_bind_group(0, scene_bind_group, &[]);
    pass.set_bind_group(
        1,
        &bind_group,
        &[
            view_uniform_offset.offset,
            previous_view_uniform_offset.offset,
        ],
    );

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if let Some(bind_group_resolve_dlss_rr_textures) = &bind_group_resolve_dlss_rr_textures {
        pass.set_bind_group(2, bind_group_resolve_dlss_rr_textures, &[]);
        pass.set_pipeline(resolve_dlss_rr_textures_pipeline);
        pass.dispatch_workgroups(dx, dy, 1);
    }

    let d = diagnostics.time_span(&mut pass, "solari_lighting/presample_light_tiles");
    pass.set_pipeline(presample_light_tiles_pipeline);
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
    pass.dispatch_workgroups_indirect(
        &solari_lighting_resources.world_cache_active_cells_dispatch,
        0,
    );

    pass.set_pipeline(sample_gi_for_world_cache_pipeline);
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

    let d = diagnostics.time_span(&mut pass, "solari_lighting/lighting");

    if let Some(bind_group_restir) = &bind_group_restir {
        pass.set_bind_group(
            1,
            bind_group_restir,
            &[
                view_uniform_offset.offset,
                previous_view_uniform_offset.offset,
            ],
        );
    }

    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    if let Some(bind_group_resolve_dlss_rr_textures) = &bind_group_resolve_dlss_rr_textures {
        pass.set_bind_group(2, bind_group_resolve_dlss_rr_textures, &[]);
    }
    pass.set_pipeline(initial_pipeline);
    pass.dispatch_workgroups(dx, dy, 1);

    if let Some(spatial_and_shade_pipeline) = spatial_and_shade_pipeline {
        pass.set_pipeline(spatial_and_shade_pipeline);
        pass.dispatch_workgroups(dx, dy, 1);
    }

    d.end(&mut pass);

    drop(pass);

    // Active cell count readback.
    diagnostics.record_u32(
        ctx.command_encoder(),
        &s.world_cache.slice(
            WORLD_CACHE_ACTIVE_CELLS_COUNT_OFFSET
                ..WORLD_CACHE_ACTIVE_CELLS_COUNT_OFFSET + size_of::<u32>() as u64,
        ),
        "solari_lighting/world_cache_active_cells_count",
    );
}

/// Initializes the Solari lighting pipelines at render startup.
pub fn init_solari_lighting_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    scene_bindings: Res<RaytracingSceneBindings>,
    asset_server: Res<AssetServer>,
) {
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "solari_lighting_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::ReadWrite),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                texture_2d(TextureSampleType::Uint),
                texture_depth_2d(),
                texture_storage_2d(TextureFormat::Rg16Float, StorageTextureAccess::ReadWrite),
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<PreviousViewData>(true),
                storage_buffer_sized(false, None),
                uniform_buffer_sized(false, None),
            ),
        ),
    );

    let bind_group_layout_restir = BindGroupLayoutDescriptor::new(
        "solari_lighting_bind_group_layout_restir",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::ReadWrite),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
                texture_2d(TextureSampleType::Uint),
                texture_depth_2d(),
                texture_storage_2d(TextureFormat::Rg16Float, StorageTextureAccess::ReadWrite),
                uniform_buffer::<ViewUniform>(true),
                uniform_buffer::<PreviousViewData>(true),
                storage_buffer_sized(false, None),
                uniform_buffer_sized(false, None),
                texture_2d(TextureSampleType::Uint),
                texture_depth_2d(),
                storage_buffer_sized(false, None),
                storage_buffer_sized(false, None),
            ),
        ),
    );

    let bind_group_layout_world_cache_active_cells_dispatch = BindGroupLayoutDescriptor::new(
        "solari_lighting_bind_group_layout_world_cache_active_cells_dispatch",
        &BindGroupLayoutEntries::single(ShaderStages::COMPUTE, storage_buffer_sized(false, None)),
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
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::Rg16Float, StorageTextureAccess::WriteOnly),
            ),
        ),
    );

    let create_pipeline = |label: &'static str,
                           entry_point: &'static str,
                           shader: Handle<Shader>,
                           restir: bool,
                           extra_bind_group: ExtraBindGroup,
                           extra_shader_defs: Vec<ShaderDefVal>| {
        let group_1 = if restir {
            &bind_group_layout_restir
        } else {
            &bind_group_layout
        };
        let mut layout = vec![scene_bindings.bind_group_layout.clone(), group_1.clone()];
        match extra_bind_group {
            ExtraBindGroup::None => {}
            ExtraBindGroup::WorldCacheDispatch => {
                layout.push(bind_group_layout_world_cache_active_cells_dispatch.clone());
            }
            #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
            ExtraBindGroup::DlssRrGuideBuffers => {
                layout.push(bind_group_layout_resolve_dlss_rr_textures.clone());
            }
        }

        let mut shader_defs = vec![ShaderDefVal::UInt(
            "WORLD_CACHE_SIZE".into(),
            WORLD_CACHE_SIZE as u32,
        )];
        if restir {
            shader_defs.push("RESTIR".into());
        }
        match extra_bind_group {
            ExtraBindGroup::None => {}
            ExtraBindGroup::WorldCacheDispatch => {
                shader_defs.push("WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER".into());
            }
            #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
            ExtraBindGroup::DlssRrGuideBuffers => {
                shader_defs.push("DLSS_RR_GUIDE_BUFFERS".into());
            }
        }
        shader_defs.extend_from_slice(&extra_shader_defs);

        pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(label.into()),
            layout,
            shader,
            shader_defs,
            entry_point: Some(entry_point.into()),
            ..default()
        })
    };

    commands.insert_resource(SolariLightingPipelines {
        bind_group_layout: bind_group_layout.clone(),
        bind_group_layout_restir: bind_group_layout_restir.clone(),
        bind_group_layout_world_cache_active_cells_dispatch:
            bind_group_layout_world_cache_active_cells_dispatch.clone(),
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        bind_group_layout_resolve_dlss_rr_textures: bind_group_layout_resolve_dlss_rr_textures
            .clone(),
        decay_world_cache_pipeline: create_pipeline(
            "solari_lighting_decay_world_cache_pipeline",
            "decay_world_cache",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_compact.wesl"),
            false,
            ExtraBindGroup::WorldCacheDispatch,
            vec![],
        ),
        compact_world_cache_single_block_pipeline: create_pipeline(
            "solari_lighting_compact_world_cache_single_block_pipeline",
            "compact_world_cache_single_block",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_compact.wesl"),
            false,
            ExtraBindGroup::WorldCacheDispatch,
            vec![],
        ),
        compact_world_cache_blocks_pipeline: create_pipeline(
            "solari_lighting_compact_world_cache_blocks_pipeline",
            "compact_world_cache_blocks",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_compact.wesl"),
            false,
            ExtraBindGroup::WorldCacheDispatch,
            vec![],
        ),
        compact_world_cache_write_active_cells_pipeline: create_pipeline(
            "solari_lighting_compact_world_cache_write_active_cells_pipeline",
            "compact_world_cache_write_active_cells",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_compact.wesl"),
            false,
            ExtraBindGroup::WorldCacheDispatch,
            vec![],
        ),
        sample_di_for_world_cache_pipeline: create_pipeline(
            "solari_lighting_sample_di_for_world_cache_pipeline",
            "sample_di",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_update.wesl"),
            false,
            ExtraBindGroup::None,
            vec![],
        ),
        sample_gi_for_world_cache_pipeline: create_pipeline(
            "solari_lighting_sample_gi_for_world_cache_pipeline",
            "sample_gi",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_update.wesl"),
            false,
            ExtraBindGroup::None,
            vec!["WORLD_CACHE_QUERY_ATOMIC_MAX_LIFETIME".into()],
        ),
        blend_new_world_cache_samples_pipeline: create_pipeline(
            "solari_lighting_blend_new_world_cache_samples_pipeline",
            "blend_new_samples",
            load_embedded_asset!(asset_server.as_ref(), "world_cache_update.wesl"),
            false,
            ExtraBindGroup::None,
            vec![],
        ),
        presample_light_tiles_pipeline: create_pipeline(
            "solari_lighting_presample_light_tiles_pipeline",
            "presample_light_tiles",
            load_embedded_asset!(asset_server.as_ref(), "presample_light_tiles.wesl"),
            false,
            ExtraBindGroup::None,
            vec![],
        ),
        restir: RestirPipelines {
            initial_and_temporal: create_pipeline(
                "solari_lighting_initial_and_temporal_pipeline",
                "initial_and_temporal",
                load_embedded_asset!(asset_server.as_ref(), "restir.wesl"),
                true,
                ExtraBindGroup::None,
                vec![],
            ),
            #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
            initial_and_temporal_with_psr: create_pipeline(
                "solari_lighting_initial_and_temporal_with_psr_pipeline",
                "initial_and_temporal",
                load_embedded_asset!(asset_server.as_ref(), "restir.wesl"),
                true,
                ExtraBindGroup::DlssRrGuideBuffers,
                vec![],
            ),
            spatial_and_shade: create_pipeline(
                "solari_lighting_spatial_and_shade_pipeline",
                "spatial_and_shade",
                load_embedded_asset!(asset_server.as_ref(), "restir.wesl"),
                true,
                ExtraBindGroup::None,
                vec!["SPATIAL_MERGE".into()],
            ),
        },
        no_restir: NoRestirPipelines {
            initial_and_shade: create_pipeline(
                "solari_lighting_initial_and_shade_pipeline",
                "initial_and_shade",
                load_embedded_asset!(asset_server.as_ref(), "no_restir.wesl"),
                false,
                ExtraBindGroup::None,
                vec![],
            ),
            #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
            initial_and_shade_with_psr: create_pipeline(
                "solari_lighting_initial_and_shade_with_psr_pipeline",
                "initial_and_shade",
                load_embedded_asset!(asset_server.as_ref(), "no_restir.wesl"),
                false,
                ExtraBindGroup::DlssRrGuideBuffers,
                vec![],
            ),
        },
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        resolve_dlss_rr_textures_pipeline: create_pipeline(
            "solari_lighting_resolve_dlss_rr_textures_pipeline",
            "resolve_dlss_rr_textures",
            load_embedded_asset!(asset_server.as_ref(), "resolve_dlss_rr_textures.wesl"),
            false,
            ExtraBindGroup::DlssRrGuideBuffers,
            vec![],
        ),
    });
}

enum ExtraBindGroup {
    None,
    WorldCacheDispatch,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
    DlssRrGuideBuffers,
}
