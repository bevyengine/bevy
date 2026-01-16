use super::{
    prepare::DlssRenderContext, Dlss, DlssFeature, DlssRayReconstructionFeature,
    DlssSuperResolutionFeature, ViewDlssRayReconstructionTextures,
};
use bevy_camera::MainPassResolutionOverride;
use bevy_core_pipeline::prepass::ViewPrepassTextures;
use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    camera::TemporalJitter,
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    renderer::{RenderAdapter, RenderContext},
    view::ViewTarget,
};
use dlss_wgpu::{
    ray_reconstruction::{
        DlssRayReconstructionRenderParameters, DlssRayReconstructionSpecularGuide,
    },
    super_resolution::{DlssSuperResolutionExposure, DlssSuperResolutionRenderParameters},
};
use std::marker::PhantomData;

#[derive(Default)]
pub struct DlssNode<F: DlssFeature>(PhantomData<F>);

impl ViewNode for DlssNode<DlssSuperResolutionFeature> {
    type ViewQuery = (
        &'static Dlss<DlssSuperResolutionFeature>,
        &'static DlssRenderContext<DlssSuperResolutionFeature>,
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
        let (Some(prepass_depth_texture), Some(prepass_motion_vectors_texture)) =
            (&prepass_textures.depth, &prepass_textures.motion_vectors)
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

        let diagnostics = render_context.diagnostic_recorder();
        let command_encoder = render_context.command_encoder();
        let mut dlss_context = dlss_context.context.lock().unwrap();

        command_encoder.push_debug_group("dlss_super_resolution");
        let time_span = diagnostics.time_span(command_encoder, "dlss_super_resolution");

        dlss_context
            .render(render_parameters, command_encoder, &adapter)
            .expect("Failed to render DLSS Super Resolution");

        time_span.end(command_encoder);
        command_encoder.pop_debug_group();

        Ok(())
    }
}

impl ViewNode for DlssNode<DlssRayReconstructionFeature> {
    type ViewQuery = (
        &'static Dlss<DlssRayReconstructionFeature>,
        &'static DlssRenderContext<DlssRayReconstructionFeature>,
        &'static MainPassResolutionOverride,
        &'static TemporalJitter,
        &'static ViewTarget,
        &'static ViewPrepassTextures,
        &'static ViewDlssRayReconstructionTextures,
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
            ray_reconstruction_textures,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let adapter = world.resource::<RenderAdapter>();
        let (Some(prepass_depth_texture), Some(prepass_motion_vectors_texture)) =
            (&prepass_textures.depth, &prepass_textures.motion_vectors)
        else {
            return Ok(());
        };

        let view_target = view_target.post_process_write();

        let render_resolution = resolution_override.0;
        let render_parameters = DlssRayReconstructionRenderParameters {
            diffuse_albedo: &ray_reconstruction_textures.diffuse_albedo.default_view,
            specular_albedo: &ray_reconstruction_textures.specular_albedo.default_view,
            normals: &ray_reconstruction_textures.normal_roughness.default_view,
            roughness: None,
            color: &view_target.source,
            depth: &prepass_depth_texture.texture.default_view,
            motion_vectors: &prepass_motion_vectors_texture.texture.default_view,
            specular_guide: DlssRayReconstructionSpecularGuide::SpecularMotionVectors(
                &ray_reconstruction_textures
                    .specular_motion_vectors
                    .default_view,
            ),
            screen_space_subsurface_scattering_guide: None, // TODO
            bias: None,                                     // TODO
            dlss_output: &view_target.destination,
            reset: dlss.reset,
            jitter_offset: -temporal_jitter.offset,
            partial_texture_size: Some(render_resolution),
            motion_vector_scale: Some(-render_resolution.as_vec2()),
        };

        let diagnostics = render_context.diagnostic_recorder();
        let command_encoder = render_context.command_encoder();
        let mut dlss_context = dlss_context.context.lock().unwrap();

        command_encoder.push_debug_group("dlss_ray_reconstruction");
        let time_span = diagnostics.time_span(command_encoder, "dlss_ray_reconstruction");

        dlss_context
            .render(render_parameters, command_encoder, &adapter)
            .expect("Failed to render DLSS Ray Reconstruction");

        time_span.end(command_encoder);
        command_encoder.pop_debug_group();

        Ok(())
    }
}
