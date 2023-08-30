use bevy_app::Plugin;
use bevy_ecs::{query::QueryItem, world::World};
use bevy_render::{
    picking::{CurrentGpuPickingBufferIndex, ExtractedGpuPickingCamera, VisibleMeshIdTextures},
    render_graph::{RenderGraphApp, RenderGraphContext, ViewNode, ViewNodeRunner},
    renderer::RenderContext,
    RenderApp,
};

use crate::core_3d::CORE_3D;

#[derive(Default)]
pub struct EntityIndexBufferCopyNode;
impl ViewNode for EntityIndexBufferCopyNode {
    type ViewQuery = (
        &'static VisibleMeshIdTextures,
        &'static ExtractedGpuPickingCamera,
    );

    fn run(
        &self,
        _graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (mesh_id_textures, gpu_picking_camera): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), bevy_render::render_graph::NodeRunError> {
        let current_buffer_index = world.resource::<CurrentGpuPickingBufferIndex>();
        gpu_picking_camera.run_node(
            render_context.command_encoder(),
            &mesh_id_textures.main.texture,
            current_buffer_index,
        );
        Ok(())
    }
}

pub struct EntityIndexBufferCopyPlugin;
impl Plugin for EntityIndexBufferCopyPlugin {
    fn build(&self, app: &mut bevy_app::App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // 3D
        use crate::core_3d::graph::node::*;
        render_app
            .add_render_graph_node::<ViewNodeRunner<EntityIndexBufferCopyNode>>(
                CORE_3D,
                ENTITY_INDEX_BUFFER_COPY,
            )
            .add_render_graph_edge(CORE_3D, UPSCALING, ENTITY_INDEX_BUFFER_COPY);
    }
}
