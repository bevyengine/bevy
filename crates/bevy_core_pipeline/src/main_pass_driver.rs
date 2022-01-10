use bevy_ecs::{
    entity::Entity,
    prelude::{QueryState, With},
    world::World,
};
use bevy_render::{
    camera::{Camera2d, Camera3d},
    render_graph::{Node, NodeRunError, RenderGraphContext, SlotValue},
    renderer::RenderContext,
};

pub struct MainPassDriverNode {
    query_camera_2d: QueryState<Entity, With<Camera2d>>,
    query_camera_3d: QueryState<Entity, With<Camera3d>>,
}

impl MainPassDriverNode {
    pub fn new(render_world: &mut World) -> Self {
        MainPassDriverNode {
            query_camera_2d: QueryState::new(render_world),
            query_camera_3d: QueryState::new(render_world),
        }
    }
}

impl Node for MainPassDriverNode {
    fn update(&mut self, world: &mut World) {
        self.query_camera_2d.update_archetypes(world);
        self.query_camera_3d.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        _render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        for camera_2d in self.query_camera_2d.iter_manual(world) {
            graph.run_sub_graph(
                crate::draw_2d_graph::NAME,
                vec![SlotValue::Entity(camera_2d)],
            )?;
        }

        for camera_3d in self.query_camera_3d.iter_manual(world) {
            graph.run_sub_graph(
                crate::draw_3d_graph::NAME,
                vec![SlotValue::Entity(camera_3d)],
            )?;
        }

        Ok(())
    }
}
