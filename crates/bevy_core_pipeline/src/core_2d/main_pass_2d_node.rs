use crate::{
    clear_color::{ClearColor, ClearColorConfig},
    core_2d::{camera_2d::Camera2d, Transparent2d},
};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    render_graph::{Node, NodeRunError, RenderGraphContext},
    render_phase::RenderPhase,
    render_resource::{LoadOp, Operations, RenderPassDescriptor},
    renderer::RenderContext,
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

impl FromWorld for MainPass2dNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            query: world.query_filtered(),
        }
    }
}

impl Node for MainPass2dNode {
    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let view_entity = graph.view_entity();
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

            let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
                label: Some("main_pass_2d"),
                color_attachments: &[Some(target.get_color_attachment(Operations {
                    load: match camera_2d.clear_color {
                        ClearColorConfig::Default => {
                            LoadOp::Clear(world.resource::<ClearColor>().0.into())
                        }
                        ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
                        ClearColorConfig::None => LoadOp::Load,
                    },
                    store: true,
                }))],
                depth_stencil_attachment: None,
            });

            if let Some(viewport) = camera.viewport.as_ref() {
                render_pass.set_camera_viewport(viewport);
            }

            transparent_phase.render(&mut render_pass, world, view_entity);
        }

        // WebGL2 quirk: if ending with a render pass with a custom viewport, the viewport isn't
        // reset for the next render pass so add an empty render pass without a custom viewport
        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
        if camera.viewport.is_some() {
            #[cfg(feature = "trace")]
            let _reset_viewport_pass_2d = info_span!("reset_viewport_pass_2d").entered();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("reset_viewport_pass_2d"),
                color_attachments: &[Some(target.get_color_attachment(Operations {
                    load: LoadOp::Load,
                    store: true,
                }))],
                depth_stencil_attachment: None,
            };

            render_context
                .command_encoder()
                .begin_render_pass(&pass_descriptor);
        }

        Ok(())
    }
}
