use crate::{ColorMaterial, Sprite, TextureAtlas, TextureAtlasSprite};
use bevy_asset::{Assets, HandleUntyped};
use bevy_reflect::TypeUuid;
use bevy_render::{
    pipeline::{
        BlendComponent, BlendFactor, BlendOperation, BlendState, ColorTargetState, ColorWrite,
        CompareFunction, DepthBiasState, DepthStencilState, FrontFace, PipelineDescriptor,
        PolygonMode, PrimitiveState, PrimitiveTopology, StencilFaceState, StencilState,
    },
    render_graph::{base, AssetRenderResourcesNode, RenderGraph, RenderResourcesNode},
    shader::{Shader, ShaderStage, ShaderStages},
    texture::TextureFormat,
};

pub const SPRITE_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 2785347840338765446);

pub const SPRITE_SHEET_PIPELINE_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(PipelineDescriptor::TYPE_UUID, 9016885805180281612);

pub fn build_sprite_sheet_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState {
                front: StencilFaceState::IGNORE,
                back: StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: DepthBiasState {
                constant: 0,
                slope_scale: 0.0,
                clamp: 0.0,
            },
        }),
        color_target_states: vec![ColorTargetState {
            format: TextureFormat::default(),
            blend: Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            }),
            write_mask: ColorWrite::ALL,
        }],
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: PolygonMode::Fill,
            clamp_depth: false,
            conservative: false,
        },
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                include_str!("sprite_sheet.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("sprite_sheet.frag"),
            ))),
        })
    }
}

pub fn build_sprite_pipeline(shaders: &mut Assets<Shader>) -> PipelineDescriptor {
    PipelineDescriptor {
        depth_stencil: Some(DepthStencilState {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilState {
                front: StencilFaceState::IGNORE,
                back: StencilFaceState::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
            bias: DepthBiasState {
                constant: 0,
                slope_scale: 0.0,
                clamp: 0.0,
            },
        }),
        color_target_states: vec![ColorTargetState {
            format: TextureFormat::default(),
            blend: Some(BlendState {
                color: BlendComponent {
                    src_factor: BlendFactor::SrcAlpha,
                    dst_factor: BlendFactor::OneMinusSrcAlpha,
                    operation: BlendOperation::Add,
                },
                alpha: BlendComponent {
                    src_factor: BlendFactor::One,
                    dst_factor: BlendFactor::One,
                    operation: BlendOperation::Add,
                },
            }),
            write_mask: ColorWrite::ALL,
        }],
        primitive: PrimitiveState {
            topology: PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: FrontFace::Ccw,
            cull_mode: None,
            polygon_mode: PolygonMode::Fill,
            clamp_depth: false,
            conservative: false,
        },
        ..PipelineDescriptor::new(ShaderStages {
            vertex: shaders.add(Shader::from_glsl(
                ShaderStage::Vertex,
                include_str!("sprite.vert"),
            )),
            fragment: Some(shaders.add(Shader::from_glsl(
                ShaderStage::Fragment,
                include_str!("sprite.frag"),
            ))),
        })
    }
}

pub mod node {
    pub const COLOR_MATERIAL: &str = "color_material";
    pub const SPRITE: &str = "sprite";
    pub const SPRITE_SHEET: &str = "sprite_sheet";
    pub const SPRITE_SHEET_SPRITE: &str = "sprite_sheet_sprite";
}

pub(crate) fn add_sprite_graph(
    graph: &mut RenderGraph,
    pipelines: &mut Assets<PipelineDescriptor>,
    shaders: &mut Assets<Shader>,
) {
    graph.add_system_node(
        node::COLOR_MATERIAL,
        AssetRenderResourcesNode::<ColorMaterial>::new(false),
    );
    graph
        .add_node_edge(node::COLOR_MATERIAL, base::node::MAIN_PASS)
        .unwrap();

    graph.add_system_node(node::SPRITE, RenderResourcesNode::<Sprite>::new(true));
    graph
        .add_node_edge(node::SPRITE, base::node::MAIN_PASS)
        .unwrap();

    graph.add_system_node(
        node::SPRITE_SHEET,
        AssetRenderResourcesNode::<TextureAtlas>::new(false),
    );

    graph.add_system_node(
        node::SPRITE_SHEET_SPRITE,
        RenderResourcesNode::<TextureAtlasSprite>::new(true),
    );

    pipelines.set_untracked(SPRITE_PIPELINE_HANDLE, build_sprite_pipeline(shaders));
    pipelines.set_untracked(
        SPRITE_SHEET_PIPELINE_HANDLE,
        build_sprite_sheet_pipeline(shaders),
    );
}
