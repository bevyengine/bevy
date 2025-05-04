use crate::{blit::BlitPipeline, upscaling::ViewUpscalingPipeline};
use bevy_color::LinearRgba;
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::{CameraOutputMode, ClearColor, ClearColorConfig, ExtractedCamera},
    frame_graph::{
        render_pass_builder::RenderPassBuilder, FrameGraph, FrameGraphTexture, ResourceRead,
        ResourceRef,
    },
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::PipelineCache,
    view::ViewTarget,
};

#[derive(Default)]
pub struct UpscalingNode;

impl ViewNode for UpscalingNode {
    type ViewQuery = (
        &'static ViewTarget,
        &'static ViewUpscalingPipeline,
        Option<&'static ExtractedCamera>,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        (target, upscaling_target, camera): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.get_resource::<PipelineCache>().unwrap();
        let blit_pipeline = world.get_resource::<BlitPipeline>().unwrap();
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

        if pipeline_cache
            .get_render_pipeline(upscaling_target.0)
            .is_none()
        {
            return Ok(());
        }

        let converted_clear_color: Option<LinearRgba> = clear_color.map(|color| color.to_linear());

        let main_texture_key = target.get_main_texture_key();

        let mut builder =
            RenderPassBuilder::new(frame_graph.create_pass_node_bulder("upscaling_pass"));

        let main_texture_read: ResourceRef<FrameGraphTexture, ResourceRead> =
            builder.read_from_board(main_texture_key)?;

        builder
            .add_raw_color_attachment(target.out_texture_color_attachment(converted_clear_color))
            .set_render_pipeline(upscaling_target.0)
            .set_bind_group(
                0,
                (
                    None,
                    blit_pipeline.texture_bind_group.clone(),
                    vec![
                        main_texture_read.into(),
                        blit_pipeline.sampler_info.clone().into(),
                    ],
                ),
                &[],
            )?
            .set_camera_viewport(camera.and_then(|camera| camera.viewport.clone()))
            .draw(0..3, 0..1);

        Ok(())
    }
}
