use crate::{
    material::StandardMaterial, nodes::LightsNode, passes::build_main_pass,
    pipelines::build_forward_pipeline,
};
use bevy_app::GetEventReader;
use bevy_asset::AssetStorage;
use bevy_render::{
    draw_target::AssignedMeshesDrawTarget,
    pipeline::PipelineDescriptor,
    render_graph::{
        nodes::{
            AssetUniformNode, CameraNode, PassNode, UniformNode, WindowSwapChainNode,
            WindowTextureNode,
        },
        RenderGraph,
    },
    shader::Shader,
    texture::{Extent3d, TextureDescriptor, TextureDimension, TextureFormat, TextureUsage},
};
use bevy_transform::prelude::LocalToWorld;
use bevy_window::{WindowCreated, WindowReference, WindowResized};
use legion::prelude::Resources;

pub trait ForwardPbrRenderGraphBuilder {
    fn add_pbr_graph(&mut self, resources: &Resources) -> &mut Self;
}

impl ForwardPbrRenderGraphBuilder for RenderGraph {
    fn add_pbr_graph(&mut self, resources: &Resources) -> &mut Self {
        self.add_system_node_named("camera", CameraNode::default(), resources);
        self.add_system_node_named(
            "local_to_world",
            UniformNode::<LocalToWorld>::new(true),
            resources,
        );
        self.add_system_node_named(
            "standard_material",
            AssetUniformNode::<StandardMaterial>::new(true),
            resources,
        );
        self.add_system_node_named("lights", LightsNode::new(10), resources);
        self.add_node_named(
            "swapchain",
            WindowSwapChainNode::new(
                WindowReference::Primary,
                resources.get_event_reader::<WindowCreated>(),
                resources.get_event_reader::<WindowResized>(),
            ),
        );
        self.add_node_named(
            "main_pass_depth_texture",
            WindowTextureNode::new(
                WindowReference::Primary,
                TextureDescriptor {
                    size: Extent3d {
                        depth: 1,
                        width: 1,
                        height: 1,
                    },
                    array_layer_count: 1,
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format: TextureFormat::Depth32Float, // PERF: vulkan docs recommend using 24 bit depth for better performance
                    usage: TextureUsage::OUTPUT_ATTACHMENT,
                },
                resources.get_event_reader::<WindowCreated>(),
                resources.get_event_reader::<WindowResized>(),
            ),
        );
        let mut shaders = resources.get_mut::<AssetStorage<Shader>>().unwrap();
        let mut pipelines = resources
            .get_mut::<AssetStorage<PipelineDescriptor>>()
            .unwrap();
        let mut main_pass = PassNode::new(build_main_pass());
        main_pass.add_pipeline(
            pipelines.add_default(build_forward_pipeline(&mut shaders)),
            vec![Box::new(AssignedMeshesDrawTarget)],
        );
        self.add_node_named("main_pass", main_pass);

        // TODO: replace these with "autowire" groups
        self.add_node_edge("camera", "main_pass").unwrap();
        self.add_node_edge("standard_material", "main_pass")
            .unwrap();
        self.add_node_edge("local_to_world", "main_pass").unwrap();
        self.add_node_edge("lights", "main_pass").unwrap();
        self.add_slot_edge(
            "swapchain",
            WindowSwapChainNode::OUT_TEXTURE,
            "main_pass",
            "color",
        )
        .unwrap();
        self.add_slot_edge(
            "main_pass_depth_texture",
            WindowTextureNode::OUT_TEXTURE,
            "main_pass",
            "depth",
        )
        .unwrap();

        self
    }
}
