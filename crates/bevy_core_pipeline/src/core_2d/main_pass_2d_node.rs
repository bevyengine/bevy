use crate::{
    clear_color::{ClearColor, ClearColorConfig},
    core_2d::{camera_2d::Camera2d, Transparent2d},
};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::{ExtractedCamera, RenderTarget},
    prelude::Image,
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo, SlotType},
    render_phase::{DrawFunctions, RenderPhase, TrackedRenderPass},
    render_resource::{
        CommandEncoderDescriptor, Extent3d, LoadOp, Operations, RenderPassDescriptor,
    },
    renderer::{RenderContext, RenderQueue},
    view::{ExtractedView, ViewTarget},
};
#[cfg(feature = "trace")]
use bevy_utils::tracing::info_span;

pub struct MainPass2dNode {
    query: QueryState<
        (
            &'static ExtractedCamera,
            &'static RenderPhase<Transparent2d>,
            &'static ViewTarget,
            &'static Camera2d,
        ),
        With<ExtractedView>,
    >,
}

impl MainPass2dNode {
    pub const IN_VIEW: &'static str = "view";

    pub fn new(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
        }
    }
}

impl Node for MainPass2dNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(MainPass2dNode::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.get_input_entity(Self::IN_VIEW)?;
        let (camera, transparent_phase, target, camera_2d) =
            if let Ok(result) = self.query.get_manual(world, view_entity) {
                result
            } else {
                // no target
                return Ok(());
            };
        {
            #[cfg(feature = "trace")]
            let _main_pass_2d = info_span!("main_pass_2d").entered();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("main_pass_2d"),
                color_attachments: &[target.get_color_attachment(Operations {
                    load: match camera_2d.clear_color {
                        ClearColorConfig::Default => {
                            LoadOp::Clear(world.resource::<ClearColor>().0.into())
                        }
                        ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
                        ClearColorConfig::None => LoadOp::Load,
                    },
                    store: true,
                })],
                depth_stencil_attachment: None,
            };

            let draw_functions = world.resource::<DrawFunctions<Transparent2d>>();

            let render_pass = render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);

            let mut draw_functions = draw_functions.write();
            let mut tracked_pass = TrackedRenderPass::new(render_pass);
            if let Some(viewport) = camera.viewport.as_ref() {
                tracked_pass.set_camera_viewport(viewport);
            }
            for item in &transparent_phase.items {
                let draw_function = draw_functions.get_mut(item.draw_function).unwrap();
                draw_function.draw(world, &mut tracked_pass, view_entity, item);
            }
        }

        // TODO: should this live here or in a separate node?
        // TODO: don't just have duplicated code between MainPass3dNode/MainPass2dNode
        if let RenderTarget::BufferedImage(image_handle, buffer_image_handle) = &camera.target {
            let gpu_images = world.get_resource::<RenderAssets<Image>>().unwrap();
            let gpu_image = gpu_images.get(&image_handle).unwrap();

            let mut encoder = render_context
                .render_device
                .create_command_encoder(&CommandEncoderDescriptor::default());

            let target_image = gpu_images.get(&buffer_image_handle).unwrap();
            encoder.copy_texture_to_texture(
                gpu_image.texture.as_image_copy(),
                target_image.texture.as_image_copy(),
                Extent3d {
                    width: gpu_image.size.x as u32,
                    height: gpu_image.size.y as u32,
                    depth_or_array_layers: 1,
                },
            );

            let render_queue = world.get_resource::<RenderQueue>().unwrap();
            render_queue.submit(std::iter::once(encoder.finish()));
        }

        // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
        // reset for the next render pass so add an empty render pass without a custom viewport
        #[cfg(feature = "webgl")]
        if camera.viewport.is_some() {
            #[cfg(feature = "trace")]
            let _reset_viewport_pass_2d = info_span!("reset_viewport_pass_2d").entered();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("reset_viewport_pass_2d"),
                color_attachments: &[target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                })],
                depth_stencil_attachment: None,
            };

            render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
        }

        Ok(())
    }
}
