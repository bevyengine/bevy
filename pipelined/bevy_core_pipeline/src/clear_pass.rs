use crate::ClearColor;
use bevy_ecs::prelude::*;
use bevy_render2::{
    camera::{CameraPlugin, ExtractedCameraNames},
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotInfo},
    render_resource::{LoadOp, Operations, RenderPassDepthStencilAttachment, RenderPassDescriptor},
    renderer::RenderContext,
    view::{ExtractedView, ViewDepthTexture, ViewTarget},
};

pub struct ClearPassNode {
    query: QueryState<(&'static ViewTarget, &'static ViewDepthTexture), With<ExtractedView>>,
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
        let extracted_cameras = world.get_resource::<ExtractedCameraNames>().unwrap();
        let view_entities: Vec<&Entity> = extracted_cameras
            .entities
            .iter()
            .filter(|(name, _)| {
                name.as_str() == CameraPlugin::CAMERA_2D || name.as_str() == CameraPlugin::CAMERA_3D
            })
            .map(|(_, entity)| entity)
            .collect();

        for view_entity in view_entities {
            let (target, depth) = self
                .query
                .get_manual(world, *view_entity)
                .expect("view entity should exist");
            let clear_color = world.get_resource::<ClearColor>().unwrap();
            let pass_descriptor = RenderPassDescriptor {
                label: Some(crate::node::CLEAR_PASS),
                color_attachments: &[target.get_color_attachment(Operations {
                    load: LoadOp::Clear(clear_color.0.into()),
                    store: true,
                })],
                depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                    view: &depth.view,
                    // NOTE: The opaque main pass clears and writes to the depth buffer.
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

        Ok(())
    }
}
