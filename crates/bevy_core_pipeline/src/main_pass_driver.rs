use bevy_ecs::world::World;
use bevy_render::{
    camera::{CameraPlugin, ExtractedCameraNames},
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotValue},
    renderer::RenderContext,
};

pub struct MainPassDriverNode;

impl Node for MainPassDriverNode {
    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let extracted_cameras = world.get_resource::<ExtractedCameraNames>().unwrap();
        if let Some(camera_2d) = extracted_cameras.entities.get(CameraPlugin::CAMERA_2D) {
            graph.run_sub_graph(
                crate::draw_2d_graph::NAME,
                vec![SlotValue::Entity(*camera_2d)],
            )?;
        }

        if let Some(camera_3d) = extracted_cameras.entities.get(CameraPlugin::CAMERA_3D) {
            graph.run_sub_graph(
                crate::draw_3d_graph::NAME,
                vec![SlotValue::Entity(*camera_3d)],
            )?;
        }

        Ok(())
    }
}
