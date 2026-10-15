use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::system::Res;
use bevy_image::ToExtents;
use bevy_math::{UVec2, Vec3Swizzles};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    extract_component::DynamicUniformIndex,
    render_resource::{ComputePass, ComputePassDescriptor, PipelineCache, RenderPassDescriptor},
    renderer::{RenderContext, ViewQuery},
    view::{ViewTarget, ViewUniformOffset},
};

use crate::{resources::GpuAtmosphere, ViewLightsUniformOffset};

use super::{
    resources::{
        AtmosphereBindGroups, AtmosphereLutPipelines, AtmosphereTextures,
        AtmosphereTransformsOffset, RenderSkyPipelineId,
    },
    CloudLayer, GpuAtmosphereSettings,
};

pub fn atmosphere_luts(
    view: ViewQuery<(
        &GpuAtmosphereSettings,
        &AtmosphereBindGroups,
        &AtmosphereTextures,
        &DynamicUniformIndex<GpuAtmosphere>,
        &DynamicUniformIndex<GpuAtmosphereSettings>,
        &AtmosphereTransformsOffset,
        &ViewUniformOffset,
        &ViewLightsUniformOffset,
        Option<&DynamicUniformIndex<CloudLayer>>,
        &super::ExtractedAtmosphere,
    )>,
    pipelines: Res<AtmosphereLutPipelines>,
    mut lut_cache: bevy_ecs::system::Local<(u64, u32)>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (
        settings,
        bind_groups,
        textures,
        atmosphere_uniforms_offset,
        settings_uniforms_offset,
        atmosphere_transforms_offset,
        view_uniforms_offset,
        lights_uniforms_offset,
        cloud_layer_uniforms_offset,
        extracted_atmosphere,
    ) = view.into_inner();

    let (
        Some(transmittance_lut_pipeline),
        Some(multiscattering_lut_pipeline),
        Some(sky_view_lut_pipeline),
        Some(aerial_view_lut_pipeline),
    ) = (
        pipeline_cache.get_compute_pipeline(pipelines.transmittance_lut),
        pipeline_cache.get_compute_pipeline(pipelines.multiscattering_lut),
        pipeline_cache.get_compute_pipeline(pipelines.sky_view_lut),
        pipeline_cache.get_compute_pipeline(pipelines.aerial_view_lut),
    )
    else {
        return;
    };

    let (cloud_shadow_map_pipeline, cloud_shadow_filter_pipeline, cloud_shadow_temporal_pipeline) = (
        pipeline_cache.get_compute_pipeline(pipelines.cloud_shadow_map),
        pipeline_cache.get_compute_pipeline(pipelines.cloud_shadow_filter),
        pipeline_cache.get_compute_pipeline(pipelines.cloud_shadow_temporal),
    );

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    fn dispatch_2d(compute_pass: &mut ComputePass, size: UVec2) {
        const WORKGROUP_SIZE: u32 = 16;
        let workgroups_x = size.x.div_ceil(WORKGROUP_SIZE);
        let workgroups_y = size.y.div_ceil(WORKGROUP_SIZE);
        compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }

    fn dispatch_2d_temporal(compute_pass: &mut ComputePass, size: UVec2) {
        const WORKGROUP_SIZE: u32 = 8;
        let workgroups_x = size.x.div_ceil(WORKGROUP_SIZE);
        let workgroups_y = size.y.div_ceil(WORKGROUP_SIZE);
        compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }

    let timing_device = ctx.render_device().clone();
    let command_encoder = ctx.command_encoder();

    // Pass 1: build all atmosphere LUTs in a single compute pass.
    {
        let mut luts_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("atmosphere_luts"),
            timestamp_writes: super::gpu_timing::compute(&timing_device, "atmosphere_luts"),
        });
        let pass_span = diagnostics.pass_span(&mut luts_pass, "atmosphere_luts");

        // Transmittance + multiscattering LUTs depend only on the atmosphere/medium and LUT
        // settings, not on the view or sun: rebuild them only when those inputs (or the LUT
        // textures) change. A periodic refresh bounds staleness for in-place medium edits.
        let lut_key = {
            use core::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            let a = extracted_atmosphere;
            for f in [
                a.inner_radius,
                a.outer_radius,
                a.ground_albedo.x,
                a.ground_albedo.y,
                a.ground_albedo.z,
            ] {
                f.to_bits().hash(&mut h);
            }
            a.medium.hash(&mut h);
            (
                settings.transmittance_lut_size,
                settings.multiscattering_lut_size,
            )
                .hash(&mut h);
            (
                settings.transmittance_lut_samples,
                settings.multiscattering_lut_dirs,
                settings.multiscattering_lut_samples,
            )
                .hash(&mut h);
            textures.transmittance_lut.texture.id().hash(&mut h);
            textures.multiscattering_lut.texture.id().hash(&mut h);
            h.finish()
        };
        let lut_static_dirty = lut_cache.0 != lut_key
            || lut_cache.1 >= 240
            || std::env::var_os("ATMOSPHERE_LUT_ALWAYS").is_some();
        if lut_static_dirty {
            *lut_cache = (lut_key, 0);
        } else {
            lut_cache.1 += 1;
        }

        if lut_static_dirty {
            // Transmittance LUT
            luts_pass.set_pipeline(transmittance_lut_pipeline);
            luts_pass.set_bind_group(
                0,
                &bind_groups.transmittance_lut,
                &[
                    atmosphere_uniforms_offset.index(),
                    settings_uniforms_offset.index(),
                ],
            );
            dispatch_2d(&mut luts_pass, settings.transmittance_lut_size);

            // Multiscattering LUT
            luts_pass.set_pipeline(multiscattering_lut_pipeline);
            luts_pass.set_bind_group(
                0,
                &bind_groups.multiscattering_lut,
                &[
                    atmosphere_uniforms_offset.index(),
                    settings_uniforms_offset.index(),
                ],
            );
            luts_pass.dispatch_workgroups(
                settings.multiscattering_lut_size.x,
                settings.multiscattering_lut_size.y,
                1,
            );
        }

        // Sky View LUT
        luts_pass.set_pipeline(sky_view_lut_pipeline);
        luts_pass.set_bind_group(
            0,
            &bind_groups.sky_view_lut,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
                atmosphere_transforms_offset.index(),
                view_uniforms_offset.offset,
                lights_uniforms_offset.offset,
            ],
        );
        dispatch_2d(&mut luts_pass, settings.sky_view_lut_size);

        // Aerial View LUT
        luts_pass.set_pipeline(aerial_view_lut_pipeline);
        luts_pass.set_bind_group(
            0,
            &bind_groups.aerial_view_lut,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
                view_uniforms_offset.offset,
                lights_uniforms_offset.offset,
            ],
        );
        dispatch_2d(&mut luts_pass, settings.aerial_view_lut_size.xy());
        pass_span.end(&mut luts_pass);
    }

    // Pass 2: cloud shadow map tracing (Unreal-style front depth + extinction stats).
    // Needed whenever clouds exist (both atmosphere modes). Kept in its own compute pass (and diagnostics span)
    // so its GPU cost shows up separately from the atmosphere LUTs in profilers.
    if cloud_layer_uniforms_offset.is_some() {
        let mut cloud_shadow_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("cloud_shadow_map"),
            timestamp_writes: super::gpu_timing::compute(&timing_device, "cloud_shadow_map"),
        });
        let pass_span = diagnostics.pass_span(&mut cloud_shadow_pass, "cloud_shadow_map");
        {
            if let (Some(cloud_shadow_map_pipeline), Some(_cloud_shadow_filter_pipeline)) =
                (cloud_shadow_map_pipeline, cloud_shadow_filter_pipeline)
            {
                if let (Some(cloud_layer_uniforms_offset), Some(cloud_shadow_map_bg)) = (
                    cloud_layer_uniforms_offset.as_ref(),
                    bind_groups.cloud_shadow_map.as_ref(),
                ) {
                    cloud_shadow_pass.set_pipeline(cloud_shadow_map_pipeline);
                    cloud_shadow_pass.set_bind_group(
                        0,
                        cloud_shadow_map_bg,
                        &[
                            atmosphere_uniforms_offset.index(),
                            settings_uniforms_offset.index(),
                            view_uniforms_offset.offset,
                            lights_uniforms_offset.offset,
                            cloud_layer_uniforms_offset.index(),
                        ],
                    );
                    // With time-slicing, one thread traces one texel per N x N tile.
                    let interleave = settings.cloud_shadow_map_update_interleave.max(1);
                    dispatch_2d(
                        &mut cloud_shadow_pass,
                        UVec2::new(
                            settings.cloud_shadow_map_size.x.div_ceil(interleave),
                            settings.cloud_shadow_map_size.y.div_ceil(interleave),
                        ),
                    );
                }
            }
        }
        pass_span.end(&mut cloud_shadow_pass);
    }

    // IMPORTANT:
    // We run the cloud shadow *filter* in a separate compute pass so the backend can insert the
    // required resource state transitions (storage-write -> sampled-read) between tracing and filtering.
    // Without this, the filter can appear to do nothing on some backends.
    if settings.cloud_shadow_map_spatial_filter_iterations > 0
        && settings.cloud_shadow_map_update_interleave <= 1
        && cloud_layer_uniforms_offset.is_some()
        && bind_groups.cloud_shadow_filter_a_to_b.is_some()
        && bind_groups.cloud_shadow_filter_b_to_a.is_some()
    {
        let Some(cloud_shadow_filter_pipeline) = cloud_shadow_filter_pipeline else {
            return;
        };
        let iters = settings.cloud_shadow_map_spatial_filter_iterations;
        let iters_even = (iters + 1) & !1;
        if iters_even > 0 {
            let mut filter_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("cloud_shadow_filter"),
                timestamp_writes: None,
            });

            filter_pass.set_pipeline(cloud_shadow_filter_pipeline);
            for i in 0..iters_even {
                let bg = if (i & 1) == 0 {
                    bind_groups.cloud_shadow_filter_a_to_b.as_ref().unwrap()
                } else {
                    bind_groups.cloud_shadow_filter_b_to_a.as_ref().unwrap()
                };
                filter_pass.set_bind_group(0, bg, &[settings_uniforms_offset.index()]);
                dispatch_2d(&mut filter_pass, settings.cloud_shadow_map_size);
            }
        }
    }

    // Temporal filter: blend current (tmp) with history, write to cloud_shadow_map.
    // Then copy cloud_shadow_map -> history for next frame.
    if settings.cloud_shadow_uses_temporal_pass() && cloud_layer_uniforms_offset.is_some() {
        if let (Some(temporal_pipeline), Some(temporal_bg)) = (
            cloud_shadow_temporal_pipeline,
            bind_groups.cloud_shadow_temporal.as_ref(),
        ) {
            let mut temporal_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("cloud_shadow_temporal"),
                timestamp_writes: super::gpu_timing::compute(
                    &timing_device,
                    "cloud_shadow_temporal",
                ),
            });
            temporal_pass.set_pipeline(temporal_pipeline);
            temporal_pass.set_bind_group(0, temporal_bg, &[]);
            dispatch_2d_temporal(&mut temporal_pass, settings.cloud_shadow_map_size);
        }

        // Copy cloud_shadow_map to history for next frame.
        let copy_size = settings.cloud_shadow_map_size.to_extents();
        command_encoder.copy_texture_to_texture(
            textures.cloud_shadow_map.texture.as_image_copy(),
            textures.cloud_shadow_map_history.texture.as_image_copy(),
            copy_size,
        );
    }
}

pub fn render_sky(
    view: ViewQuery<(
        &ExtractedCamera,
        &AtmosphereBindGroups,
        &ViewTarget,
        &DynamicUniformIndex<GpuAtmosphere>,
        &DynamicUniformIndex<GpuAtmosphereSettings>,
        &AtmosphereTransformsOffset,
        &ViewUniformOffset,
        &ViewLightsUniformOffset,
        Option<&DynamicUniformIndex<CloudLayer>>,
        &RenderSkyPipelineId,
        Option<&MainPassResolutionOverride>,
        &AtmosphereTextures,
    )>,
    pipeline_cache: Res<PipelineCache>,
    pipelines: Res<AtmosphereLutPipelines>,
    noise_dump: (
        Option<Res<super::perlin_worley_noise::PerlinWorleyNoiseTexture>>,
        Option<Res<super::detail_noise::DetailNoiseTexture>>,
        Option<Res<super::fbm_noise::FbmNoiseTexture>>,
    ),
    mut ctx: RenderContext,
) {
    let (
        camera,
        atmosphere_bind_groups,
        view_target,
        atmosphere_uniforms_offset,
        settings_uniforms_offset,
        atmosphere_transforms_offset,
        view_uniforms_offset,
        lights_uniforms_offset,
        cloud_layer_uniforms_offset,
        render_sky_pipeline_id,
        resolution_override,
        textures,
    ) = view.into_inner();

    let Some(render_sky_pipeline) = pipeline_cache.get_render_pipeline(render_sky_pipeline_id.0)
    else {
        return;
    }; //TODO: warning

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let dump_device = ctx.render_device().clone();
    let timing_device = ctx.render_device().clone();
    let command_encoder = ctx.command_encoder();

    // Debug readback (CLOUD_DUMP_DIR): textures as they are when this frame's sky starts, i.e.
    // this frame's shadow map / noise and the *previous* frame's cloud trace + reconstruct.
    let dump_frame = super::debug_dump::begin_frame();
    if let Some(frame) = dump_frame {
        use bevy_math::UVec3;
        let d = |enc: &mut _,
                 name: &str,
                 tex: &bevy_render::render_resource::Texture,
                 mip: u32,
                 size: UVec3,
                 bpt: u32,
                 ext: &str| {
            super::debug_dump::dump(&dump_device, enc, frame, name, tex, mip, size, bpt, ext);
        };
        let s2 = |t: &bevy_render::render_resource::Texture| UVec3::new(t.width(), t.height(), 1);
        for (name, t) in [
            ("shadow_map", &textures.cloud_shadow_map.texture),
            ("shadow_raw", &textures.cloud_shadow_map_tmp.texture),
            ("clouds_trace_prev", &textures.clouds_trace.texture),
            ("clouds_recon_prev", &textures.clouds.texture),
            ("ms_lut", &textures.multiscattering_lut.texture),
            ("sky_view_lut", &textures.sky_view_lut.texture),
        ] {
            d(command_encoder, name, t, 0, s2(t), 8, "rgba16f");
        }
        let (pw, det, wx) = &noise_dump;
        if let Some(pw) = pw {
            d(
                command_encoder,
                "shape_noise_mip0",
                &pw.texture.texture,
                0,
                pw.size,
                8,
                "rgba16f",
            );
        }
        if let Some(det) = det {
            d(
                command_encoder,
                "detail_noise_mip0",
                &det.texture.texture,
                0,
                det.size,
                8,
                "rgba16f",
            );
        }
        if let Some(wx) = wx {
            d(
                command_encoder,
                "weather",
                &wx.texture.texture,
                0,
                UVec3::new(wx.size.x, wx.size.y, 1),
                8,
                "rgba16f",
            );
        }
    }

    // Dedicated low-res cloud raymarch (design.md §1.2). Its output is composited by render_sky.
    if let (Some(cloud_layer_offset), Some(bind_group), Some(pipeline)) = (
        cloud_layer_uniforms_offset.as_ref(),
        atmosphere_bind_groups.clouds_raymarch.as_ref(),
        pipeline_cache.get_compute_pipeline(pipelines.clouds_raymarch),
    ) {
        let mut clouds_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("clouds_raymarch"),
            timestamp_writes: super::gpu_timing::compute(&timing_device, "clouds_raymarch"),
        });
        let pass_span = diagnostics.pass_span(&mut clouds_pass, "clouds_raymarch");
        clouds_pass.set_pipeline(pipeline);
        clouds_pass.set_bind_group(
            0,
            bind_group,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
                atmosphere_transforms_offset.index(),
                view_uniforms_offset.offset,
                lights_uniforms_offset.offset,
                cloud_layer_offset.index(),
            ],
        );
        let size = textures.clouds_trace.texture.size();
        clouds_pass.dispatch_workgroups(size.width.div_ceil(8), size.height.div_ceil(8), 1);
        pass_span.end(&mut clouds_pass);
        drop(clouds_pass);

        // Checkerboard reconstruction into this frame's half-res buffer (ping-pong with history).
        if let (Some(reconstruct_bind_group), Some(reconstruct_pipeline)) = (
            atmosphere_bind_groups.clouds_reconstruct.as_ref(),
            pipeline_cache.get_compute_pipeline(pipelines.clouds_reconstruct),
        ) {
            let mut pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("clouds_reconstruct"),
                timestamp_writes: super::gpu_timing::compute(&timing_device, "clouds_reconstruct"),
            });
            let pass_span = diagnostics.pass_span(&mut pass, "clouds_reconstruct");
            pass.set_pipeline(reconstruct_pipeline);
            pass.set_bind_group(
                0,
                reconstruct_bind_group,
                &[
                    atmosphere_uniforms_offset.index(),
                    view_uniforms_offset.offset,
                ],
            );
            let size = textures.clouds.texture.size();
            pass.dispatch_workgroups(size.width.div_ceil(8), size.height.div_ceil(8), 1);
            pass_span.end(&mut pass);
        }
    }

    let mut render_sky_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("render_sky"),
        color_attachments: &[Some(view_target.get_color_attachment())],
        depth_stencil_attachment: None,
        timestamp_writes: super::gpu_timing::render(&timing_device, "render_sky"),
        occlusion_query_set: None,
        multiview_mask: None,
    });
    let pass_span = diagnostics.pass_span(&mut render_sky_pass, "render_sky");

    if let Some(viewport) =
        Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
    {
        render_sky_pass.set_viewport(
            viewport.physical_position.x as f32,
            viewport.physical_position.y as f32,
            viewport.physical_size.x as f32,
            viewport.physical_size.y as f32,
            viewport.depth.start,
            viewport.depth.end,
        );
    }

    render_sky_pass.set_pipeline(render_sky_pipeline);

    // Select correct bind group + dynamic offsets based on whether the view has CloudLayer.
    // No-cloud variant omits the CloudLayer binding entirely.
    //
    // If cloud bind group isn't ready yet, skip this pass (pipeline cache will catch up next frame).
    let (bind_group, cloud_layer_offset) = match (
        cloud_layer_uniforms_offset.as_ref(),
        atmosphere_bind_groups.render_sky_clouds.as_ref(),
    ) {
        (Some(offset), Some(bg)) => (bg, Some(offset)),
        _ => (&atmosphere_bind_groups.render_sky_no_clouds, None),
    };

    let offsets_no_clouds = [
        atmosphere_uniforms_offset.index(),
        settings_uniforms_offset.index(),
        atmosphere_transforms_offset.index(),
        view_uniforms_offset.offset,
        lights_uniforms_offset.offset,
    ];

    if let Some(cloud_layer_offset) = cloud_layer_offset {
        let offsets_clouds = [
            offsets_no_clouds[0],
            offsets_no_clouds[1],
            offsets_no_clouds[2],
            offsets_no_clouds[3],
            offsets_no_clouds[4],
            cloud_layer_offset.index(),
        ];
        render_sky_pass.set_bind_group(0, bind_group, &offsets_clouds);
    } else {
        render_sky_pass.set_bind_group(0, bind_group, &offsets_no_clouds);
    }
    render_sky_pass.draw(0..3, 0..1);
    pass_span.end(&mut render_sky_pass);
    drop(render_sky_pass);
    // Debug readback of the final linear (pre-TAA, pre-tonemap) scene color.
    if let Some(frame) = dump_frame {
        let t = view_target.main_texture();
        super::debug_dump::dump(
            &dump_device,
            command_encoder,
            frame,
            "scene_hdr",
            t,
            0,
            bevy_math::UVec3::new(t.width(), t.height(), 1),
            8,
            "rgba16f",
        );
    }
    super::gpu_timing::end_frame(&timing_device, command_encoder);
}
