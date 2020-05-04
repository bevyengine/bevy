use bevy_asset::{AssetStorage, Handle};
use bevy_render::{
    draw_target::AssignedMeshesDrawTarget,
    pipeline::{state_descriptors::*, PipelineDescriptor},
    render_graph::{nodes::{UniformNode, PassNode}, RenderGraph},
    shader::{Shader, ShaderStage, ShaderStages},
    texture::TextureFormat, base_render_graph,
};
use legion::prelude::Resources;
use crate::Rect;

pub const UI_PIPELINE_HANDLE: Handle<PipelineDescriptor> =
    Handle::from_bytes([
        163, 238, 40, 24, 156, 49, 73, 203, 156, 189, 249, 55, 133, 242, 116, 51,
    ]);

pub fn build_ui_pipeline(shaders: &mut AssetStorage<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
        }),
        depth_stencil_state: Some(DepthStencilStateDescriptor {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: CompareFunction::Always,
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

pub trait UiRenderGraphBuilder {
    fn add_ui_graph(&mut self, resources: &Resources) -> &mut Self;
}

impl UiRenderGraphBuilder for RenderGraph {
    fn add_ui_graph(&mut self, resources: &Resources) -> &mut Self {
        self.add_system_node_named("rect", UniformNode::<Rect>::new(false), resources);
        self.add_node_edge("rect", base_render_graph::node::MAIN_PASS).unwrap();
        let mut pipelines = resources.get_mut::<AssetStorage<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<AssetStorage<Shader>>().unwrap();
        pipelines.add_with_handle(UI_PIPELINE_HANDLE, build_ui_pipeline(&mut shaders));
        let main_pass: &mut PassNode = self.get_node_mut(base_render_graph::node::MAIN_PASS).unwrap();
        main_pass.add_pipeline(UI_PIPELINE_HANDLE, vec![Box::new(AssignedMeshesDrawTarget)]);
        self
    }
}