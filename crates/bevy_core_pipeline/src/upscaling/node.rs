use crate::{blit::BlitPipeline, upscaling::ViewUpscalingPipeline};
use bevy_camera::{CameraOutputMode, ClearColor, ClearColorConfig};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{BindGroup, PipelineCache, RenderPassDescriptor, TextureViewId},
    renderer::RenderContext,
    view::ViewTarget,
};
use std::sync::Mutex;

#[derive(Default)]
pub struct UpscalingNode {
    cached_texture_bind_group: Mutex<Option<(TextureViewId, BindGroup)>>,
}

impl ViewNode for UpscalingNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewUpscalingPipeline,
        Option<&'static ExtractedCamera>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, upscaling_target, camera): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let blit_pipeline = world.resource::<BlitPipeline>();
        let clear_color_global = world.resource::<ClearColor>();

        let diagnostics = render_context.diagnostic_recorder();

        let clear_color = if let Some(camera) = camera {
            match camera.output_mode {
                CameraOutputMode::Write { clear_color, .. } => clear_color,
                CameraOutputMode::Skip => return Ok(()),
            }
        } else {
            ClearColorConfig::Default
        };
        let clear_color = match clear_color {
            ClearColorConfig::Default => Some(clear_color_global.0),
            ClearColorConfig::Custom(color) => Some(color),
            ClearColorConfig::None => None,
        };
        let converted_clear_color = clear_color.map(Into::into);
        // texture to be upscaled to the output texture
        let main_texture_view = target.main_texture_view();

        let mut cached_bind_group = self.cached_texture_bind_group.lock().unwrap();
        let bind_group = match &mut *cached_bind_group {
            Some((id, bind_group)) if main_texture_view.id() == *id => bind_group,
            cached_bind_group => {
                let bind_group = blit_pipeline
                    .create_bind_group(render_context.render_device(), main_texture_view);

                let (_, bind_group) =
                    cached_bind_group.insert((main_texture_view.id(), bind_group));
                bind_group
            }
        };

        let Some(pipeline) = pipeline_cache.get_render_pipeline(upscaling_target.0) else {
            return Ok(());
        };

        let pass_descriptor = RenderPassDescriptor {
            label: Some("upscaling"),
            color_attachments: &[Some(
                target.out_texture_color_attachment(converted_clear_color),
            )],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);
        let pass_span = diagnostics.pass_span(&mut render_pass, "upscaling");

        if let Some(camera) = camera
            && let Some(viewport) = &camera.viewport
        {
            let size = viewport.physical_size;
            let position = viewport.physical_position;
            render_pass.set_scissor_rect(position.x, position.y, size.x, size.y);
        }

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, bind_group, &[]);
        render_pass.draw(0..3, 0..1);

        pass_span.end(&mut render_pass);

        Ok(())
    }
}
