use bevy_asset::{Assets, Handle};
use bevy_render::{
    base_render_graph,
    pipeline::{state_descriptors::*, PipelineDescriptor},
    render_graph::{nodes::{PassNode, CameraNode, RenderResourcesNode}, RenderGraph},
    shader::{Shader, ShaderStage, ShaderStages},
    texture::TextureFormat, ActiveCameras,
};
use legion::prelude::Resources;
use crate::Node;

pub const UI_PIPELINE_HANDLE: Handle<PipelineDescriptor> =
    Handle::from_u128(323432002226399387835192542539754486265);

pub fn build_ui_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Cw,
            cull_mode: CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        depth_stencil_state: Some(DepthStencilStateDescriptor {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::Less,
            stencil_front: StencilStateFaceDescriptor::IGNORE,
            stencil_back: StencilStateFaceDescriptor::IGNORE,
            stencil_read_mask: 0,
            stencil_write_mask: 0,
        }),
        color_states: vec![ColorStateDescriptor {
            format: TextureFormat::Bgra8UnormSrgb,
            color_blend: BlendDescriptor {
                src_factor: BlendFactor::SrcAlpha,
                dst_factor: BlendFactor::OneMinusSrcAlpha,
                operation: BlendOperation::Add,
            },
            alpha_blend: BlendDescriptor {
                src_factor: BlendFactor::One,
                dst_factor: BlendFactor::One,
                operation: BlendOperation::Add,
            },
            write_mask: ColorWrite::ALL,
        }],
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                include_str!("ui.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("ui.frag"),
            ))),
        })
    }
}

pub mod node {
    pub const UI_CAMERA: &'static str = "ui_camera";
    pub const NODE: &'static str = "node";
}

pub mod camera {
    pub const UI_CAMERA: &'static str = "UiCamera";
}

pub trait UiRenderGraphBuilder {
    fn add_ui_graph(&mut self, resources: &Resources) -> &mut Self;
}

impl UiRenderGraphBuilder for RenderGraph {
    fn add_ui_graph(&mut self, resources: &Resources) -> &mut Self {
        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        pipelines.set(UI_PIPELINE_HANDLE, build_ui_pipeline(&mut shaders));

        // setup ui camera
        self.add_system_node(node::UI_CAMERA, CameraNode::new(camera::UI_CAMERA));
        self.add_node_edge(node::UI_CAMERA, base_render_graph::node::MAIN_PASS)
            .unwrap();
        self.add_system_node(node::NODE, RenderResourcesNode::<Node>::new(true));
        self.add_node_edge(node::NODE, base_render_graph::node::MAIN_PASS)
            .unwrap();
        let mut active_cameras = resources.get_mut::<ActiveCameras>().unwrap();
        let main_pass_node: &mut PassNode = self.get_node_mut(base_render_graph::node::MAIN_PASS).unwrap();
        main_pass_node.add_camera(camera::UI_CAMERA);
        active_cameras.add(camera::UI_CAMERA);
        self
    }
}
