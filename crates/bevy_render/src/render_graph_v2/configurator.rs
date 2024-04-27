use super::{RenderGraph, RenderGraphBuilder, RenderGraphPersistentResources};
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
pub struct RenderGraphConfigurator(Box<dyn Fn(RenderGraphBuilder) + Send + Sync + 'static>);

impl RenderGraphConfigurator {
    pub fn new(f: impl Fn(RenderGraphBuilder) + Send + Sync + 'static) -> Self {
        Self(Box::new(f))
    }
}

/// Configures the [`RenderGraph`] based on entities with the [`RenderGraphConfigurator`] component.
pub fn setup_view_render_graph_nodes(world: &mut World) {
    //1. new RenderGraph<'g>
    //2. scope RenderGraphPersistentResources
    //3. run all configurators
    //4. create pipelines
    //5. run graph

    todo!()
}
