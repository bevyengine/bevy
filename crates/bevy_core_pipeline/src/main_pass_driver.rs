use bevy_ecs::world::World;
use bevy_render::{
    camera::{CameraPlugin, ExtractedCameraNames},
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotValue, RunSubGraphs},
    renderer::RenderContext,
};

pub struct MainPassDriverNode;

impl Node for MainPassDriverNode {

    fn queue_graphs(&self, graph: &RenderGraphContext, world: &World) -> Result<RunSubGraphs, NodeRunError> {
        let extracted_cameras = world.get_resource::<ExtractedCameraNames>().unwrap();
        let mut sub_graph_runs = RunSubGraphs::default();

        if let Some(camera_2d) = extracted_cameras.entities.get(CameraPlugin::CAMERA_2D) {
            sub_graph_runs.run(
                crate::draw_2d_graph::NAME,
                vec![("view", SlotValue::Entity(*camera_2d))],
            );
        }

        if let Some(camera_3d) = extracted_cameras.entities.get(CameraPlugin::CAMERA_3D) {
            sub_graph_runs.run(
                crate::draw_3d_graph::NAME,
                vec![("view", SlotValue::Entity(*camera_3d))],
            );
        }

        Ok(sub_graph_runs)
    }

}
