use crate::upscaling::ViewUpscalingTextureBlitter;
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::{CameraOutputMode, ClearColor, ClearColorConfig, ExtractedCamera},
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    renderer::RenderContext,
    texture_blitter::TextureBlitterRenderPass,
    view::ViewTarget,
};
use wgpu::LoadOp;

#[derive(Default)]
pub struct UpscalingNode;

impl ViewNode for UpscalingNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewUpscalingTextureBlitter,
        Option<&'static ExtractedCamera>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (target, upscaling_texture_blitter, camera): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let clear_color_global = world.get_resource::<ClearColor>().unwrap();

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
        let upscaled_texture = target.main_texture_view();

        // We need to use this function because it determines if the clear color will be used or
        // not
        let out_texture_attachment = target.out_texture_color_attachment(converted_clear_color);

        let mut render_pass = TextureBlitterRenderPass::default();
        // Set the clear color if needed this is determined by `out_texture_color_attachment()`
        if let LoadOp::Clear(color) = out_texture_attachment.ops.load {
            render_pass.clear_color = Some(color);
        }

        // Set the scissor rect of the texture blitter pass
        if let Some(camera) = camera {
            if let Some(viewport) = &camera.viewport {
                let size = viewport.physical_size;
                let position = viewport.physical_position;
                render_pass.scissor_rect = Some((position.x, position.y, size.x, size.y));
            }
        }

        // Do the blit to the output texture
        render_context.blit_with_render_pass(
            &upscaling_texture_blitter.0,
            upscaled_texture,
            out_texture_attachment.view,
            &render_pass,
        );

        Ok(())
    }
}
