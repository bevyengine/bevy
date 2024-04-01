use super::RenderGraph;
use bevy_ecs::{component::Component, entity::Entity, world::World};

/// Component for automatically configuring the [`RenderGraph`] each frame for an entity.
///
/// When attached to an entity, each frame as part of the [`setup_view_render_graph_nodes`] system,
/// the function contained within this component will be called.
///
/// The function will be provided the entity, the render world, and the render graph, and should create any
/// resources (via [`RenderGraph::create_resource`]) and add any nodes (via [`RenderGraph::add_node`]) to the render graph
/// that it wants for the current frame.
#[derive(Component)]
pub struct RenderGraphConfigurator(
    pub(crate) Box<dyn Fn(Entity, &World, &mut RenderGraph) + Send + Sync + 'static>,
);

impl RenderGraphConfigurator {
    pub fn new(f: impl Fn(Entity, &World, &mut RenderGraph) + Send + Sync + 'static) -> Self {
        Self(Box::new(f))
    }
}

/// Configures the [`RenderGraph`] based on entities with the [`RenderGraphConfigurator`] component.
pub fn setup_view_render_graph_nodes(world: &mut World) {
    world.resource_scope::<RenderGraph, _>(|world, mut render_graph| {
        // TODO: Probably want to cache the QueryState
        for (view_entity, configurator) in world
            .query::<(Entity, &RenderGraphConfigurator)>()
            .iter(world)
        {
            (configurator.0)(view_entity, world, &mut render_graph);
        }
    });
}
