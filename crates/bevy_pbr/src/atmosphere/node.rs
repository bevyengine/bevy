use bevy_ecs::{query::QueryItem, system::lifetimeless::Read, world::World};
use bevy_math::{UVec2, Vec3Swizzles};
use bevy_render::{
    extract_component::DynamicUniformIndex,
    frame_graph::{ComputePassBuilder, FrameGraph},
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
    render_resource::PipelineCache,
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
        frame_graph: &mut FrameGraph,
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
        let (Some(_), Some(_), Some(_), Some(_)) = (
            pipeline_cache.get_compute_pipeline(pipelines.transmittance_lut),
            pipeline_cache.get_compute_pipeline(pipelines.multiscattering_lut),
            pipeline_cache.get_compute_pipeline(pipelines.sky_view_lut),
            pipeline_cache.get_compute_pipeline(pipelines.aerial_view_lut),
        ) else {
            return Ok(());
        };

        let mut pass_builder = frame_graph.create_pass_builder("atmosphere_luts_node");

        let mut compute_pass_builder =
            pass_builder.create_compute_pass_builder("atmosphere_luts_pass");

        fn dispatch_2d(builder: &mut ComputePassBuilder, size: UVec2) {
            const WORKGROUP_SIZE: u32 = 16;
            let workgroups_x = size.x.div_ceil(WORKGROUP_SIZE);
            let workgroups_y = size.y.div_ceil(WORKGROUP_SIZE);
            builder.dispatch_workgroups(workgroups_x, workgroups_y, 1);
        }

        // Transmittance LUT
        compute_pass_builder
            .set_compute_pipeline(pipelines.transmittance_lut)
            .set_bind_group_handle(
                0,
                &bind_groups.transmittance_lut,
                &[
                    atmosphere_uniforms_offset.index(),
                    settings_uniforms_offset.index(),
                ],
            );
        dispatch_2d(&mut compute_pass_builder, settings.transmittance_lut_size);

        // Multiscattering LUT
        compute_pass_builder
            .set_compute_pipeline(pipelines.multiscattering_lut)
            .set_bind_group_handle(
                0,
                &bind_groups.multiscattering_lut,
                &[
                    atmosphere_uniforms_offset.index(),
                    settings_uniforms_offset.index(),
                ],
            )
            .dispatch_workgroups(
                settings.multiscattering_lut_size.x,
                settings.multiscattering_lut_size.y,
                1,
            );

        // Sky View LUT
        compute_pass_builder
            .set_compute_pipeline(pipelines.sky_view_lut)
            .set_bind_group_handle(
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
        dispatch_2d(&mut compute_pass_builder, settings.sky_view_lut_size);

        // Aerial View LUT

        compute_pass_builder
            .set_compute_pipeline(pipelines.aerial_view_lut)
            .set_bind_group_handle(
                0,
                &bind_groups.aerial_view_lut,
                &[
                    atmosphere_uniforms_offset.index(),
                    settings_uniforms_offset.index(),
                    view_uniforms_offset.offset,
                    lights_uniforms_offset.offset,
                ],
            );

        dispatch_2d(
            &mut compute_pass_builder,
            settings.aerial_view_lut_size.xy(),
        );

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
        frame_graph: &mut FrameGraph,
        (
            atmosphere_bind_groups,
            view_target,
            atmosphere_uniforms_offset,
            settings_uniforms_offset,
            atmosphere_transforms_offset,
            view_uniforms_offset,
            lights_uniforms_offset,
            render_sky_pipeline_id,
        ): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let Some(_) = pipeline_cache.get_render_pipeline(render_sky_pipeline_id.0) else {
            return Ok(());
        }; //TODO: warning

        let mut pass_builder = frame_graph.create_pass_builder("render_sky_node");

        let color_attachment = view_target.get_color_attachment(&mut pass_builder);

        pass_builder
            .create_render_pass_builder("render_sky_pass")
            .add_color_attachment(color_attachment)
            .set_render_pipeline(render_sky_pipeline_id.0)
            .set_bind_group_handle(
                0,
                &atmosphere_bind_groups.render_sky,
                &[
                    atmosphere_uniforms_offset.index(),
                    settings_uniforms_offset.index(),
                    atmosphere_transforms_offset.index(),
                    view_uniforms_offset.offset,
                    lights_uniforms_offset.offset,
                ],
            )
            .draw(0..3, 0..1);

        Ok(())
    }
}
