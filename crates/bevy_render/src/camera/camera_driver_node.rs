use crate::{
    camera::{ExtractedCamera, NormalizedRenderTarget, SortedCameras},
    render_graph::{Node, NodeRunError, RenderGraphContext},
    renderer::RenderContext,
    view::ExtractedWindows,
};
use bevy_ecs::{prelude::QueryState, world::World};
use bevy_utils::HashSet;
use wgpu::{LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor};

pub struct CameraDriverNode {
    cameras: QueryState<&'static ExtractedCamera>,
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
        let sorted_cameras = world.resource::<SortedCameras>();
        let windows = world.resource::<ExtractedWindows>();
        let mut camera_windows = HashSet::new();
        for sorted_camera in &sorted_cameras.0 {
            let Ok(camera) = self.cameras.get_manual(world, sorted_camera.entity) else {
                continue;
            };

            let mut run_graph = true;
            if let Some(NormalizedRenderTarget::Window(window_ref)) = camera.target {
                let window_entity = window_ref.entity();
                if windows.windows.get(&window_entity).is_some() {
                    camera_windows.insert(window_entity);
                } else {
                    // The window doesn't exist anymore so we don't need to run the graph
                    run_graph = false;
                }
            }
            if run_graph {
                graph.run_sub_graph(
                    camera.render_graph.clone(),
                    vec![],
                    Some(sorted_camera.entity),
                )?;
            }
        }

        // wgpu (and some backends) require doing work for swap chains if you call `get_current_texture()` and `present()`
        // This ensures that Bevy doesn't crash, even when there are no cameras (and therefore no work submitted).
        for (id, window) in world.resource::<ExtractedWindows>().iter() {
            if camera_windows.contains(id) {
                continue;
            }

            let Some(swap_chain_texture) = &window.swap_chain_texture_view else {
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
                .command_encoder()
                .begin_render_pass(&pass_descriptor);
        }

        Ok(())
    }
}
