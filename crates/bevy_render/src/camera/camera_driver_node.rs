use crate::{
    camera::{ExtractedCamera, RenderTarget},
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotValue},
    renderer::RenderContext,
    view::ExtractedWindows,
};
use bevy_ecs::{entity::Entity, prelude::QueryState, world::World};
use bevy_utils::{tracing::warn, HashSet};
use wgpu::{LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor};

pub struct CameraDriverNode {
    cameras: QueryState<(Entity, &'static ExtractedCamera)>,
}

impl CameraDriverNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            cameras: world.query(),
        }
    }
}

impl Node for CameraDriverNode {
    fn update(&mut self, world: &mut World) {
        self.cameras.update_archetypes(world);
    }
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let mut sorted_cameras = self
            .cameras
            .iter_manual(world)
            .map(|(e, c)| (e, c.priority, c.target.clone()))
            .collect::<Vec<_>>();
        // sort by priority and ensure within a priority, RenderTargets of the same type are packed together
        sorted_cameras.sort_by(|(_, p1, t1), (_, p2, t2)| match p1.cmp(p2) {
            std::cmp::Ordering::Equal => t1.cmp(t2),
            ord => ord,
        });
        let mut camera_windows = HashSet::new();
        let mut previous_priority_target = None;
        let mut ambiguities = HashSet::new();
        for (entity, priority, target) in sorted_cameras {
            let new_priority_target = (priority, target);
            if let Some(previous_priority_target) = previous_priority_target {
                if previous_priority_target == new_priority_target {
                    ambiguities.insert(new_priority_target.clone());
                }
            }
            previous_priority_target = Some(new_priority_target);
            if let Ok((_, camera)) = self.cameras.get_manual(world, entity) {
                if let RenderTarget::Window(id) = camera.target {
                    camera_windows.insert(id);
                }
                graph
                    .run_sub_graph(camera.render_graph.clone(), vec![SlotValue::Entity(entity)])?;
            }
        }

        if !ambiguities.is_empty() {
            warn!(
                "Camera priority ambiguities detected for active cameras with the following priorities: {:?}. \
                To fix this, ensure there is exactly one Camera entity spawned with a given priority for a given RenderTarget. \
                Ambiguities should be resolved because either (1) multiple active cameras were spawned accidentally, which will \
                result in rendering multiple instances of the scene or (2) for cases where multiple active cameras is intentional, \
                ambiguities could result in unpredictable render results.",
                ambiguities
            );
        }

        // wgpu (and some backends) require doing work for swap chains if you call `get_current_texture()` and `present()`
        // This ensures that Bevy doesn't crash, even when there are no cameras (and therefore no work submitted).
        for (id, window) in world.resource::<ExtractedWindows>().iter() {
            if camera_windows.contains(id) {
                continue;
            }

            let Some(swap_chain_texture) = &window.swap_chain_texture else {
                continue;
            };

            #[cfg(feature = "trace")]
            let _span = bevy_utils::tracing::info_span!("no_camera_clear_pass").entered();
            let pass_descriptor = RenderPassDescriptor {
                label: Some("no_camera_clear_pass"),
                color_attachments: &[Some(RenderPassColorAttachment {
                    view: swap_chain_texture,
                    resolve_target: None,
                    ops: Operations {
                        load: LoadOp::Clear(wgpu::Color::BLACK),
                        store: true,
                    },
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
