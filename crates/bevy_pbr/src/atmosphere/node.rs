use bevy_ecs::{query::QueryItem, system::lifetimeless::Read, world::World};
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::DynamicUniformIndex,
    render_graph::{NodeRunError, RenderGraphContext, RenderLabel, ViewNode},
    render_resource::{
        ComputePassDescriptor, Operations, PipelineCache, RenderPassColorAttachment,
        RenderPassDescriptor,
    },
    renderer::RenderContext,
    view::ViewUniformOffset,
};

use crate::MeshViewBindGroup;

use super::{
    resources::{AtmosphereBindGroups, AtmospherePipelines, AtmosphereTextures},
    Atmosphere,
};

#[derive(PartialEq, Eq, Debug, Copy, Clone, Hash, RenderLabel)]
pub(super) struct SkyLabel;

#[derive(Default)]
pub(super) struct SkyNode {}

impl ViewNode for SkyNode {
    type ViewQuery = (
        Read<AtmosphereTextures>,
        Read<AtmosphereBindGroups>,
        Read<ViewUniformOffset>,
        Read<MeshViewBindGroup>,
        Read<DynamicUniformIndex<Atmosphere>>,
    );

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            textures,
            bind_groups,
            view_uniform_offset,
            mesh_view_bind_group,
            atmosphere_uniform_offset,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipelines = world.resource::<AtmospherePipelines>();
        let pipeline_cache = world.resource::<PipelineCache>();
        let (
            Some(transmittance_lut_pipeline),
            Some(multiscattering_lut_pipeline),
            Some(sky_view_lut_pipeline),
            Some(aerial_view_lut_pipeline),
        ) = (
            pipeline_cache.get_render_pipeline(pipelines.transmittance_lut),
            pipeline_cache.get_compute_pipeline(pipelines.multiscattering_lut),
            pipeline_cache.get_render_pipeline(pipelines.sky_view_lut),
            pipeline_cache.get_compute_pipeline(pipelines.aerial_view_lut),
        )
        else {
            //TODO: warning
            return Ok(());
        };

        let mut commands = render_context.command_encoder();

        commands.push_debug_group("sky");

        {
            let mut transmittance_lut_pass = commands.begin_render_pass(&RenderPassDescriptor {
                label: Some("transmittance_lut_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &textures.transmittance_lut.default_view,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            transmittance_lut_pass.set_pipeline(transmittance_lut_pipeline); //TODO: MESH VIEW BIND GROUP
            transmittance_lut_pass.set_bind_group(
                0,
                &bind_groups.transmittance_lut,
                &[atmosphere_uniform_offset.index()],
            );
            transmittance_lut_pass.draw(0..3, 0..1);
        }

        {
            let mut multiscattering_lut_pass =
                commands.begin_compute_pass(&ComputePassDescriptor {
                    label: Some("multiscatttering_lut_pass"),
                    timestamp_writes: None,
                });
            multiscattering_lut_pass.set_pipeline(multiscattering_lut_pipeline);
            multiscattering_lut_pass.set_bind_group(
                0,
                &bind_groups.multiscattering_lut,
                &[atmosphere_uniform_offset.index()],
            );
            multiscattering_lut_pass.dispatch_workgroups(todo!(), todo!(), todo!());
        }

        {
            let mut sky_view_lut_pass = commands.begin_render_pass(&RenderPassDescriptor {
                label: Some("transmittance_lut_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: &textures.transmittance_lut.default_view,
                    resolve_target: None,
                    ops: Operations::default(),
                })],
                depth_stencil_attachment: None,
                ..Default::default()
            });
            sky_view_lut_pass.set_pipeline(transmittance_lut_pipeline); //TODO: MESH VIEW BIND GROUP
            sky_view_lut_pass.set_bind_group(
                0,
                &bind_groups.sky_view_lut,
                &[atmosphere_uniform_offset.index()],
            );
            sky_view_lut_pass.draw(0..3, 0..1);
        }

        {
            let mut aerial_view_lut_pass = commands.begin_compute_pass(&ComputePassDescriptor {
                label: Some("multiscatttering_lut_pass"),
                timestamp_writes: None,
            });
            aerial_view_lut_pass.set_pipeline(multiscattering_lut_pipeline);
            aerial_view_lut_pass.set_bind_group(
                0,
                &bind_groups.aerial_view_lut,
                &[atmosphere_uniform_offset.index()],
            );
            aerial_view_lut_pass.dispatch_workgroups(todo!(), todo!(), todo!());
        }

        render_context.command_encoder().pop_debug_group();
        Ok(())
    }
}
