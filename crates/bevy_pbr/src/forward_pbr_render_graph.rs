use crate::{material::StandardMaterial, nodes::LightsNode, pipelines::build_forward_pipeline};
use bevy_asset::Assets;
use bevy_render::{
    base_render_graph,
    pipeline::PipelineDescriptor,
    render_graph::{
        nodes::{AssetUniformNode, PassNode, UniformNode},
        RenderGraph,
    },
    shader::Shader,
};
use bevy_transform::prelude::Transform;
use legion::prelude::Resources;

pub mod node {
    pub const TRANSFORM: &str = "transform";
    pub const STANDARD_MATERIAL: &str = "standard_material";
    pub const LIGHTS: &str = "lights";
}

pub mod uniform {
    pub const LIGHTS: &str = "Lights";
}

pub trait ForwardPbrRenderGraphBuilder {
    fn add_pbr_graph(&mut self, resources: &Resources) -> &mut Self;
}

impl ForwardPbrRenderGraphBuilder for RenderGraph {
    fn add_pbr_graph(&mut self, resources: &Resources) -> &mut Self {
        self.add_system_node(node::TRANSFORM, UniformNode::<Transform>::new(true));
        self.add_system_node(
            node::STANDARD_MATERIAL,
            AssetUniformNode::<StandardMaterial>::new(true),
        );
        self.add_system_node(node::LIGHTS, LightsNode::new(10));
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        {
            let main_pass: &mut PassNode = self
                .get_node_mut(base_render_graph::node::MAIN_PASS)
                .unwrap();
            main_pass.add_pipeline(pipelines.add_default(build_forward_pipeline(&mut shaders)));
        }

        // TODO: replace these with "autowire" groups
        self.add_node_edge(node::STANDARD_MATERIAL, base_render_graph::node::MAIN_PASS)
            .unwrap();
        self.add_node_edge(node::TRANSFORM, base_render_graph::node::MAIN_PASS)
            .unwrap();
        self.add_node_edge(node::LIGHTS, base_render_graph::node::MAIN_PASS)
            .unwrap();
        self
    }
}
