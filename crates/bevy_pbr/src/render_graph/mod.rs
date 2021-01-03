mod forward_pipeline;
mod lights_node;

use bevy_ecs::world::World;
pub use forward_pipeline::*;
pub use lights_node::*;

/// the names of pbr graph nodes
pub mod node {
    pub const TRANSFORM: &str = "transform";
    pub const STANDARD_MATERIAL: &str = "standard_material";
    pub const LIGHTS: &str = "lights";
}

/// the names of pbr uniforms
pub mod uniform {
    pub const LIGHTS: &str = "Lights";
}

use crate::prelude::StandardMaterial;
use bevy_asset::Assets;
use bevy_render::{
    pipeline::PipelineDescriptor,
    render_graph::{base, AssetRenderResourcesNode, RenderGraph, RenderResourcesNode},
    shader::Shader,
};
use bevy_transform::prelude::GlobalTransform;

pub(crate) fn add_pbr_graph(world: &mut World) {
    {
        let mut graph = world.get_resource_mut::<RenderGraph>().unwrap();
        graph.add_system_node(
            node::TRANSFORM,
            RenderResourcesNode::<GlobalTransform>::new(true),
        );
        graph.add_system_node(
            node::STANDARD_MATERIAL,
            AssetRenderResourcesNode::<StandardMaterial>::new(true),
        );
        graph.add_system_node(node::LIGHTS, LightsNode::new(10));

        // TODO: replace these with "autowire" groups
        graph
            .add_node_edge(node::STANDARD_MATERIAL, base::node::MAIN_PASS)
            .unwrap();
        graph
            .add_node_edge(node::TRANSFORM, base::node::MAIN_PASS)
            .unwrap();
        graph
            .add_node_edge(node::LIGHTS, base::node::MAIN_PASS)
            .unwrap();
    }
    let forward_pipeline =
        build_forward_pipeline(&mut world.get_resource_mut::<Assets<Shader>>().unwrap());
    let mut pipelines = world
        .get_resource_mut::<Assets<PipelineDescriptor>>()
        .unwrap();
    pipelines.set_untracked(FORWARD_PIPELINE_HANDLE, forward_pipeline);
}
