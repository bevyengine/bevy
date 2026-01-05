use bevy_camera::{MainPassResolutionOverride, Viewport};
use bevy_ecs::{query::QueryItem, system::lifetimeless::Read, world::World};
use bevy_math::{UVec2, Vec3Swizzles};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    extract_component::DynamicUniformIndex,
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
    render_resource::{ComputePass, ComputePassDescriptor, PipelineCache, RenderPassDescriptor},
    renderer::RenderContext,
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

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, RenderLabel)]
pub enum AtmosphereNode {
    RenderLuts,
    RenderSky,
    Environment,
}

#[derive(Default)]
pub(super) struct AtmosphereLutsNode {}

impl ViewNode for AtmosphereLutsNode {
    type ViewQuery = (
        Read<GpuAtmosphereSettings>,
        Read<AtmosphereBindGroups>,
        Read<DynamicUniformIndex<GpuAtmosphere>>,
        Read<DynamicUniformIndex<GpuAtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<DynamicUniformIndex<CloudLayer>>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            settings,
            bind_groups,
            atmosphere_uniforms_offset,
            settings_uniforms_offset,
            atmosphere_transforms_offset,
            view_uniforms_offset,
            lights_uniforms_offset,
            cloud_layer_uniforms_offset,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipelines = world.resource::<AtmosphereLutPipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let (
            Some(transmittance_lut_pipeline),
            Some(multiscattering_lut_pipeline),
            Some(sky_view_lut_pipeline),
            Some(aerial_view_lut_pipeline),
            Some(cloud_shadow_map_pipeline),
            Some(cloud_shadow_filter_pipeline),
        ) = (
            pipeline_cache.get_compute_pipeline(pipelines.transmittance_lut),
            pipeline_cache.get_compute_pipeline(pipelines.multiscattering_lut),
            pipeline_cache.get_compute_pipeline(pipelines.sky_view_lut),
            pipeline_cache.get_compute_pipeline(pipelines.aerial_view_lut),
            pipeline_cache.get_compute_pipeline(pipelines.cloud_shadow_map),
            pipeline_cache.get_compute_pipeline(pipelines.cloud_shadow_filter),
        )
        else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let command_encoder = render_context.command_encoder();

        fn dispatch_2d(compute_pass: &mut ComputePass, size: UVec2) {
            const WORKGROUP_SIZE: u32 = 16;
            let workgroups_x = size.x.div_ceil(WORKGROUP_SIZE);
            let workgroups_y = size.y.div_ceil(WORKGROUP_SIZE);
            compute_pass.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Pass 1: build all LUTs (+ cloud shadow map tracing) in a single compute pass.
        // This scope is important: `luts_pass` holds a mutable borrow of the command encoder.
        {
            let mut luts_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                label: Some("atmosphere_luts"),
                timestamp_writes: None,
            });
            let pass_span = diagnostics.pass_span(&mut luts_pass, "atmosphere_luts");

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
                luts_pass.set_pipeline(cloud_shadow_map_pipeline);
                luts_pass.set_bind_group(
                    0,
                    &bind_groups.cloud_shadow_map,
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

            pass_span.end(&mut luts_pass);
        }

        // IMPORTANT:
        // We run the cloud shadow *filter* in a separate compute pass so the backend can insert the
        // required resource state transitions (storage-write -> sampled-read) between tracing and filtering.
        // Without this, the filter can appear to do nothing on some backends.
        if settings.rendering_method == 1 && settings.cloud_shadow_map_spatial_filter_iterations > 0 {
            // Spatial filtering (ping-pong), Unreal-style:
            // - Blur max optical depth in transmittance space
            // - Keep depth mostly unfiltered
            //
            // Note: to keep the final texture consistent for sampling, we run an *even* number of passes,
            // alternating A->B then B->A so we always end in A (`cloud_shadow_map`).
            let iters = settings.cloud_shadow_map_spatial_filter_iterations;
            let iters_even = (iters + 1) & !1;
            if iters_even > 0 {
                let mut filter_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("cloud_shadow_filter"),
                    timestamp_writes: None,
                });
                let filter_span = diagnostics.pass_span(&mut filter_pass, "cloud_shadow_filter");

                filter_pass.set_pipeline(cloud_shadow_filter_pipeline);
                for i in 0..iters_even {
                    let bg = if (i & 1) == 0 {
                        &bind_groups.cloud_shadow_filter_a_to_b
                    } else {
                        &bind_groups.cloud_shadow_filter_b_to_a
                    };
                    filter_pass.set_bind_group(0, bg, &[settings_uniforms_offset.index()]);
                    dispatch_2d(&mut filter_pass, settings.cloud_shadow_map_size);
                }

                filter_span.end(&mut filter_pass);
            }
        }

        Ok(())
    }
}

#[derive(Default)]
pub(super) struct RenderSkyNode;

impl ViewNode for RenderSkyNode {
    type ViewQuery = (
        Read<ExtractedCamera>,
        Read<AtmosphereBindGroups>,
        Read<ViewTarget>,
        Read<DynamicUniformIndex<GpuAtmosphere>>,
        Read<DynamicUniformIndex<GpuAtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<DynamicUniformIndex<CloudLayer>>,
        Read<RenderSkyPipelineId>,
        Option<Read<MainPassResolutionOverride>>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
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
        ): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(render_sky_pipeline) =
            pipeline_cache.get_render_pipeline(render_sky_pipeline_id.0)
        else {
            return Ok(());
        }; //TODO: warning

        let diagnostics = render_context.diagnostic_recorder();

        let mut render_sky_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("render_sky"),
            color_attachments: &[Some(view_target.get_color_attachment())],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        let pass_span = diagnostics.pass_span(&mut render_sky_pass, "render_sky");

        if let Some(viewport) =
            Viewport::from_viewport_and_override(camera.viewport.as_ref(), resolution_override)
        {
            render_sky_pass.set_camera_viewport(&viewport);
        }

        render_sky_pass.set_render_pipeline(render_sky_pipeline);
        render_sky_pass.set_bind_group(
            0,
            &atmosphere_bind_groups.render_sky,
            &[
                atmosphere_uniforms_offset.index(),
                settings_uniforms_offset.index(),
                atmosphere_transforms_offset.index(),
                view_uniforms_offset.offset,
                lights_uniforms_offset.offset,
                cloud_layer_uniforms_offset.index(),
            ],
        );
        render_sky_pass.draw(0..3, 0..1);

        pass_span.end(&mut render_sky_pass);

        Ok(())
    }
}
