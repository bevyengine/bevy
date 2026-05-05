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
    GpuAtmosphereSettings,
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

    let command_encoder = ctx.command_encoder();

    let mut luts_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
        label: Some("atmosphere_luts"),
        timestamp_writes: None,
    });

    fn dispatch_2d(compute_pass: &mut ComputePass, size: UVec2) {
        const WORKGROUP_SIZE: u32 = 16;
        let workgroups_x = size.x.div_ceil(WORKGROUP_SIZE);
        let workgroups_y = size.y.div_ceil(WORKGROUP_SIZE);
        compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
    }

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
    render_sky_pass.set_bind_group(
        0,
        &atmosphere_bind_groups.render_sky,
        &[
            atmosphere_uniforms_offset.index(),
            settings_uniforms_offset.index(),
            atmosphere_transforms_offset.index(),
            view_uniforms_offset.offset,
            lights_uniforms_offset.offset,
        ],
    );
    render_sky_pass.draw(0..3, 0..1);
}
