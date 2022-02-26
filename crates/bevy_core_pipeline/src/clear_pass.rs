use std::collections::HashSet;

use crate::{ClearColor, RenderTargetClearColors};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::{ExtractedCamera, RenderTarget},
    prelude::Image,
    render_asset::RenderAssets,
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo},
    render_resource::{
        LoadOp, Operations, RenderPassColorAttachment, RenderPassDepthStencilAttachment,
        RenderPassDescriptor,
    },
    renderer::RenderContext,
    view::{ExtractedView, ExtractedWindows, ViewDepthTexture, ViewTarget},
};

pub struct ClearPassNode {
    query: QueryState<
        (
            &'static ViewTarget,
            Option<&'static ViewDepthTexture>,
            Option<&'static ExtractedCamera>,
        ),
        With<ExtractedView>,
    >,
}

impl ClearPassNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            query: QueryState::new(world),
        }
    }
}

impl Node for ClearPassNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![]
    }

    fn update(&mut self, world: &mut World) {
        self.query.update_archetypes(world);
    }

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let mut cleared_targets = HashSet::new();
        let clear_color = world.resource::<ClearColor>();
        let render_target_clear_colors = world.resource::<RenderTargetClearColors>();

        // This gets all ViewTargets and ViewDepthTextures and clears its attachments
        // TODO: This has the potential to clear the same target multiple times, if there
        // are multiple views drawing to the same target. This should be fixed when we make
        // clearing happen on "render targets" instead of "views" (see the TODO below for more context).
        for (target, depth, camera) in self.query.iter_manual(world) {
            let mut color = &clear_color.0;
            if let Some(camera) = camera {
                cleared_targets.insert(&camera.target);
                if let Some(target_color) = render_target_clear_colors.get(&camera.target) {
                    color = target_color;
                }
            }
            let pass_descriptor = RenderPassDescriptor {
                label: Some("clear_pass"),
                color_attachments: &[target.get_color_attachment(Operations {
                    load: LoadOp::Clear((*color).into()),
                    store: true,
                })],
                depth_stencil_attachment: depth.map(|depth| RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    depth_ops: Some(Operations {
                        load: LoadOp::Clear(0.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            };

            render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
        }

        // TODO: This is a hack to ensure we don't call present() on frames without any work,
        // which will cause panics. The real fix here is to clear "render targets" directly
        // instead of "views". This should be removed once full RenderTargets are implemented.
        let windows = world.resource::<ExtractedWindows>();
        let images = world.resource::<RenderAssets<Image>>();
        for target in render_target_clear_colors.colors.keys().cloned().chain(
            windows
                .values()
                .map(|window| RenderTarget::Window(window.id)),
        ) {
            // skip windows that have already been cleared
            if cleared_targets.contains(&target) {
                continue;
            }
            let pass_descriptor = RenderPassDescriptor {
                label: Some("clear_pass"),
                color_attachments: &[RenderPassColorAttachment {
                    view: target.get_texture_view(windows, images).unwrap(),
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(
                            (*render_target_clear_colors
                                .get(&target)
                                .unwrap_or(&clear_color.0))
                            .into(),
                        ),
                        store: true,
                    },
                }],
                depth_stencil_attachment: None,
            };

            render_context
                .command_encoder
                .begin_render_pass(&pass_descriptor);
        }

        Ok(())
    }
}
