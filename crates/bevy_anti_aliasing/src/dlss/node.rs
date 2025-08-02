use super::{prepare::ViewDlssSuperResolution, Dlss};
use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    camera::{MainPassResolutionOverride, TemporalJitter},
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    renderer::{RenderAdapter, RenderContext},
    view::ViewTarget,
};
use dlss_wgpu::super_resolution::{
    DlssSuperResolutionExposure, DlssSuperResolutionRenderParameters,
};

#[derive(Default)]
pub struct DlssNode;

impl ViewNode for DlssNode {
    type ViewQuery = (
        &'static Dlss,
        &'static ViewDlssSuperResolution,
        &'static MainPassResolutionOverride,
        &'static TemporalJitter,
        &'static ViewTarget,
        &'static ViewPrepassTextures,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            dlss,
            dlss_context,
            resolution_override,
            temporal_jitter,
            view_target,
            prepass_textures,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let adapter = world.resource::<RenderAdapter>();
        let (Some(prepass_motion_vectors_texture), Some(prepass_depth_texture)) =
            (&prepass_textures.motion_vectors, &prepass_textures.depth)
        else {
            return Ok(());
        };

        let view_target = view_target.post_process_write();

        let render_resolution = resolution_override.0;
        let render_parameters = DlssSuperResolutionRenderParameters {
            color: &view_target.source,
            depth: &prepass_depth_texture.texture.default_view,
            motion_vectors: &prepass_motion_vectors_texture.texture.default_view,
            exposure: DlssSuperResolutionExposure::Automatic, // TODO
            bias: None,                                       // TODO
            dlss_output: &view_target.destination,
            reset: dlss.reset,
            jitter_offset: -temporal_jitter.offset,
            partial_texture_size: Some(render_resolution),
            motion_vector_scale: Some(-render_resolution.as_vec2()),
        };

        let command_encoder = render_context.command_encoder();
        let mut dlss_context = dlss_context.context.lock().unwrap();

        command_encoder.push_debug_group("dlss");
        dlss_context
            .render(render_parameters, command_encoder, &adapter)
            .expect("Failed to render DLSS");
        command_encoder.pop_debug_group();

        Ok(())
    }
}
