use crate::{blit::BlitPipeline, upscaling::ViewUpscalingPipeline};
use bevy_color::{Color, LinearRgba};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    camera::{CameraDriverNode, CameraOutputMode, ClearColor, ClearColorConfig, ExtractedCamera},
    frame_graph::{
        BindGroupEntryRef, BindGroupRef, BindingResourceRef, ColorAttachmentRef, FrameGraph,
        RenderPass, TextureViewInfo, TextureViewRef,
    },
    render_graph::{NodeRunError, RenderGraphContext, ViewNode},
    render_resource::{LoadOp, Operations, PipelineCache, StoreOp},
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
        graph: &mut RenderGraphContext,
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
        let view_entity = graph.view_entity();

        let main_texture_key = target.get_main_texture_key(view_entity);

        let mut builder = frame_graph.create_pass_node_bulder("upscaling_pass");

        let Some(main_texture_handle) = builder.read_from_board(&main_texture_key) else {
            return Ok(());
        };
        let main_texture_read = builder.read(main_texture_handle);

        let camera_texure_key = CameraDriverNode::get_camera_texure_key(view_entity);

        let Some(camera_texure_handle) = builder.read_from_board(&camera_texure_key) else {
            return Ok(());
        };

        let camera_texure_handle_read = builder.read(camera_texure_handle);

        let mut render_pass = RenderPass::default();

        render_pass.add_color_attachment(ColorAttachmentRef {
            view_ref: TextureViewRef {
                texture_ref: camera_texure_handle_read,
                desc: TextureViewInfo::default(),
            },
            resolve_target: None,
            ops: target.out_texture_color_attachment_operations(converted_clear_color),
        });

        render_pass.set_bind_group(
            0,
            &BindGroupRef {
                label: None,
                layout: blit_pipeline.texture_bind_group.clone(),
                entries: vec![
                    BindGroupEntryRef {
                        binding: 0,
                        resource: BindingResourceRef::TextureView {
                            texture_ref: main_texture_read,
                            texture_view_info: TextureViewInfo::default(),
                        },
                    },
                    BindGroupEntryRef {
                        binding: 1,
                        resource: BindingResourceRef::Sampler(blit_pipeline.sampler.clone()),
                    },
                ],
            },
            &[],
        );

        if let Some(camera) = camera {
            if let Some(viewport) = &camera.viewport {
                let size = viewport.physical_size;
                let position = viewport.physical_position;
                render_pass.set_scissor_rect(position.x, position.y, size.x, size.y);
            }
        }

        render_pass.set_render_pipeline(upscaling_target.0);
        render_pass.draw(0..3, 0..1);

        builder.set_pass(render_pass);

        Ok(())
    }
}
