use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::system::Res;
use bevy_math::{UVec2, Vec3Swizzles};
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::DynamicUniformIndex,
    render_resource::{ComputePass, ComputePassDescriptor, PipelineCache, RenderPassDescriptor},
    renderer::{RenderContext, ViewQuery},
    view::{ViewTarget, ViewUniformOffset},
};

use crate::{resources::GpuAtmosphere, ViewLightsUniformOffset};

use super::{
    resources::{
        AtmosphereBindGroups, AtmosphereLutPipelines, AtmosphereTransformsOffset,
        RenderSkyPipelineId,
    },
    CloudLayer, GpuAtmosphereSettings,
};

pub fn atmosphere_luts(
    view: ViewQuery<(
        &GpuAtmosphereSettings,
        &AtmosphereBindGroups,
        &DynamicUniformIndex<GpuAtmosphere>,
        &DynamicUniformIndex<GpuAtmosphereSettings>,
        &AtmosphereTransformsOffset,
        &ViewUniformOffset,
        &ViewLightsUniformOffset,
        Option<&DynamicUniformIndex<CloudLayer>>,
    )>,
    pipelines: Res<AtmosphereLutPipelines>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let (
        settings,
        bind_groups,
        atmosphere_uniforms_offset,
        settings_uniforms_offset,
        atmosphere_transforms_offset,
        view_uniforms_offset,
        lights_uniforms_offset,
        cloud_layer_uniforms_offset,
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

    let (cloud_shadow_map_pipeline, cloud_shadow_filter_pipeline) = (
        pipeline_cache.get_compute_pipeline(pipelines.cloud_shadow_map),
        pipeline_cache.get_compute_pipeline(pipelines.cloud_shadow_filter),
    );

    fn dispatch_2d(compute_pass: &mut ComputePass, size: UVec2) {
        const WORKGROUP_SIZE: u32 = 16;
        let workgroups_x = size.x.div_ceil(WORKGROUP_SIZE);
        let workgroups_y = size.y.div_ceil(WORKGROUP_SIZE);
        compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }

    let command_encoder = ctx.command_encoder();

    // Pass 1: build all LUTs (+ cloud shadow map tracing) in a single compute pass.
    {
        let mut luts_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("atmosphere_luts"),
            timestamp_writes: None,
        });

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

        // Cloud shadow map (Unreal-style front depth + extinction stats)
        // Only needed for the Raymarched mode.
        if settings.rendering_method == 1 {
            if let (Some(cloud_shadow_map_pipeline), Some(_cloud_shadow_filter_pipeline)) =
                (cloud_shadow_map_pipeline, cloud_shadow_filter_pipeline)
            {
                if let (Some(cloud_layer_uniforms_offset), Some(cloud_shadow_map_bg)) = (
                    cloud_layer_uniforms_offset.as_ref(),
                    bind_groups.cloud_shadow_map.as_ref(),
                ) {
                    luts_pass.set_pipeline(cloud_shadow_map_pipeline);
                    luts_pass.set_bind_group(
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
                    dispatch_2d(&mut luts_pass, settings.cloud_shadow_map_size);
                }
            }
        }
    }

    // IMPORTANT:
    // We run the cloud shadow *filter* in a separate compute pass so the backend can insert the
    // required resource state transitions (storage-write -> sampled-read) between tracing and filtering.
    // Without this, the filter can appear to do nothing on some backends.
    if settings.rendering_method == 1
        && settings.cloud_shadow_map_spatial_filter_iterations > 0
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
    )>,
    pipeline_cache: Res<PipelineCache>,
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
    ) = view.into_inner();

    let Some(render_sky_pipeline) = pipeline_cache.get_render_pipeline(render_sky_pipeline_id.0)
    else {
        return;
    }; //TODO: warning

    let command_encoder = ctx.command_encoder();

    let mut render_sky_pass = command_encoder.begin_render_pass(&RenderPassDescriptor {
        label: Some("render_sky"),
        color_attachments: &[Some(view_target.get_color_attachment())],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

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
}
