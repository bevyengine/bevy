use bevy_ecs::{query::QueryItem, system::lifetimeless::Read, world::World};
use bevy_math::{UVec2, Vec3Swizzles};
use bevy_render::{
    diagnostic::RecordDiagnostics,
    extract_component::DynamicUniformIndex,
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
    render_resource::{ComputePass, ComputePassDescriptor, PipelineCache, RenderPassDescriptor},
    renderer::RenderContext,
    view::{ViewTarget, ViewUniformOffset},
};

use crate::ViewLightsUniformOffset;

use super::{
    resources::{
        AtmosphereBindGroups, AtmosphereLutPipelines, AtmosphereTransformsOffset,
        RenderSkyPipelineId,
    },
    Atmosphere, AtmosphereSettings,
};

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, RenderLabel)]
pub enum AtmosphereNode {
    RenderLuts,
    RenderSky,
}

#[derive(Default)]
pub(super) struct AtmosphereLutsNode {}

impl ViewNode for AtmosphereLutsNode {
    type ViewQuery = (
        Read<AtmosphereSettings>,
        Read<AtmosphereBindGroups>,
        Read<DynamicUniformIndex<Atmosphere>>,
        Read<DynamicUniformIndex<AtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
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
        ) = (
            pipeline_cache.get_compute_pipeline(pipelines.transmittance_lut),
            pipeline_cache.get_compute_pipeline(pipelines.multiscattering_lut),
            pipeline_cache.get_compute_pipeline(pipelines.sky_view_lut),
            pipeline_cache.get_compute_pipeline(pipelines.aerial_view_lut),
        )
        else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();

        let command_encoder = render_context.command_encoder();

        let mut luts_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some("atmosphere_luts"),
            timestamp_writes: None,
        });
        let pass_span = diagnostics.time_span(&mut luts_pass, "atmosphere_luts");

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

        pass_span.end(&mut luts_pass);

        Ok(())
    }
}

#[derive(Default)]
pub(super) struct RenderSkyNode;

impl ViewNode for RenderSkyNode {
    type ViewQuery = (
        Read<AtmosphereBindGroups>,
        Read<ViewTarget>,
        Read<DynamicUniformIndex<Atmosphere>>,
        Read<DynamicUniformIndex<AtmosphereSettings>>,
        Read<AtmosphereTransformsOffset>,
        Read<ViewUniformOffset>,
        Read<ViewLightsUniformOffset>,
        Read<RenderSkyPipelineId>,
    );

    fn run<'w>(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            atmosphere_bind_groups,
            view_target,
            atmosphere_uniforms_offset,
            settings_uniforms_offset,
            atmosphere_transforms_offset,
            view_uniforms_offset,
            lights_uniforms_offset,
            render_sky_pipeline_id,
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

        let mut render_sky_pass =
            render_context
                .command_encoder()
                .begin_render_pass(&RenderPassDescriptor {
                    label: Some("render_sky"),
                    color_attachments: &[Some(view_target.get_color_attachment())],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
        let pass_span = diagnostics.pass_span(&mut render_sky_pass, "render_sky");

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

        pass_span.end(&mut render_sky_pass);

        Ok(())
    }
}
