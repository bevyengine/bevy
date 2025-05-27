use crate::{
    camera::{ClearColor, ExtractedCamera, NormalizedRenderTarget},
    render_graph::{Node, NodeRunError, RenderGraphContext, RenderLabel, RenderSubGraph},
    renderer::RenderContext,
    view::ExtractedWindows,
};
use bevy_ecs::{
    entity::ContainsEntity, prelude::QueryState, system::lifetimeless::Read, world::World,
};
use bevy_platform::collections::HashSet;
use wgpu::{LoadOp, Operations, RenderPassColorAttachment, RenderPassDescriptor, StoreOp};

use super::{CompositedBy, Compositor, RenderGraphDriver, Views};

pub struct RunCompositorsNode {}

// TODO:
// - [ ] setup compositor graph structure, and defer to view render graph
// - [ ] extraction and such
// - [x] module structure. This all probably shouldn't still live in `Camera`.
// - [x] move `ComputedCameraValues` around. merge with Frustum?
// - [ ] investigate utility camera query data
// - [ ] fix event dispatch
// - [ ] fix relationship hooks
// - [ ] fix everything else oh god

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, RenderSubGraph)]
pub struct CompositorGraph;

#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, RenderLabel)]
pub struct RenderViews;

pub struct RenderViewsNode {
    compositors: QueryState<(Read<Compositor>, Read<Views>)>,
    views: QueryState<(Read<ExtractedView>, Read<RenderGraphDriver>)>,
}

impl RenderViewsNode {
    pub fn new(world: &mut World) -> Self {
        Self {
            compositors: world.query(),
            cameras: world.query(),
        }
    }
}

impl Node for RenderViewsNode {
    fn update(&mut self, world: &mut World) {
        self.cameras.update_archetypes(world);
    }
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        // let sorted_cameras = world.resource::<SortedCameras>();
        // let windows = world.resource::<ExtractedWindows>();
        // let mut camera_windows = <HashSet<_>>::default();
        // for sorted_camera in &sorted_cameras.0 {
        //     let Ok(camera) = self.cameras.get_manual(world, sorted_camera.entity) else {
        //         continue;
        //     };
        //
        //     let mut run_graph = true;
        //     if let Some(NormalizedRenderTarget::Window(window_ref)) = camera.target {
        //         let window_entity = window_ref.entity();
        //         if windows
        //             .windows
        //             .get(&window_entity)
        //             .is_some_and(|w| w.physical_width > 0 && w.physical_height > 0)
        //         {
        //             camera_windows.insert(window_entity);
        //         } else {
        //             // The window doesn't exist anymore or zero-sized so we don't need to run the graph
        //             run_graph = false;
        //         }
        //     }
        //     if run_graph {
        //         graph.run_sub_graph(camera.render_graph, vec![], Some(sorted_camera.entity))?;
        //     }
        // }
        //
        // let clear_color_global = world.resource::<ClearColor>();
        //
        // // wgpu (and some backends) require doing work for swap chains if you call `get_current_texture()` and `present()`
        // // This ensures that Bevy doesn't crash, even when there are no cameras (and therefore no work submitted).
        // for (id, window) in world.resource::<ExtractedWindows>().iter() {
        //     if camera_windows.contains(id) {
        //         continue;
        //     }
        //
        //     let Some(swap_chain_texture) = &window.swap_chain_texture_view else {
        //         continue;
        //     };
        //
        //     #[cfg(feature = "trace")]
        //     let _span = tracing::info_span!("no_camera_clear_pass").entered();
        //     let pass_descriptor = RenderPassDescriptor {
        //         label: Some("no_camera_clear_pass"),
        //         color_attachments: &[Some(RenderPassColorAttachment {
        //             view: swap_chain_texture,
        //             resolve_target: None,
        //             ops: Operations {
        //                 load: LoadOp::Clear(clear_color_global.to_linear().into()),
        //                 store: StoreOp::Store,
        //             },
        //         })],
        //         depth_stencil_attachment: None,
        //         timestamp_writes: None,
        //         occlusion_query_set: None,
        //     };
        //
        //     render_context
        //         .command_encoder()
        //         .begin_render_pass(&pass_descriptor);
        // }
        //
        // Ok(())
        todo!()
    }
}
