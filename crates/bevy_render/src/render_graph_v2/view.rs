use super::RenderGraph;
use bevy_ecs::{component::Component, entity::Entity, world::World};

#[derive(Component)]
pub struct ViewRenderGraphConfigurator(
    pub Box<dyn Fn(Entity, &World, &mut RenderGraph) + Send + Sync>,
);

pub fn setup_view_render_graph_nodes(world: &mut World) {
    world.resource_scope::<RenderGraph, _>(|world, mut render_graph| {
        // TODO: Probably want to cache the QueryState
        for (view_entity, configurator) in world
            .query::<(Entity, &ViewRenderGraphConfigurator)>()
            .iter(world)
        {
            (configurator.0)(view_entity, world, &mut render_graph);
        }
    });
}
