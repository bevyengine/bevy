use crate::{ColorMaterial, Sprite, TextureAtlas, TextureAtlasSprite};
use bevy_asset::{Assets, HandleUntyped};
use bevy_ecs::Resources;
use bevy_reflect::TypeUuid;
use bevy_render::{
    pipeline::{
        BlendDescriptor, BlendFactor, BlendOperation, ColorStateDescriptor, ColorWrite,
        CompareFunction, CullMode, DepthStencilStateDescriptor, FrontFace, PipelineDescriptor,
        RasterizationStateDescriptor, StencilStateDescriptor, StencilStateFaceDescriptor,
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
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        depth_stencil_state: Some(DepthStencilStateDescriptor {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilStateDescriptor {
                front: StencilStateFaceDescriptor::IGNORE,
                back: StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
        }),
        color_states: vec![ColorStateDescriptor {
            format: TextureFormat::default(),
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
        rasterization_state: Some(RasterizationStateDescriptor {
            front_face: FrontFace::Ccw,
            cull_mode: CullMode::None,
            depth_bias: 0,
            depth_bias_slope_scale: 0.0,
            depth_bias_clamp: 0.0,
            clamp_depth: false,
        }),
        depth_stencil_state: Some(DepthStencilStateDescriptor {
            format: TextureFormat::Depth32Float,
            depth_write_enabled: true,
            depth_compare: CompareFunction::LessEqual,
            stencil: StencilStateDescriptor {
                front: StencilStateFaceDescriptor::IGNORE,
                back: StencilStateFaceDescriptor::IGNORE,
                read_mask: 0,
                write_mask: 0,
            },
        }),
        color_states: vec![ColorStateDescriptor {
            format: TextureFormat::default(),
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

pub trait SpriteRenderGraphBuilder {
    fn add_sprite_graph(&mut self, resources: &Resources) -> &mut Self;
}

impl SpriteRenderGraphBuilder for RenderGraph {
    fn add_sprite_graph(&mut self, resources: &Resources) -> &mut Self {
        self.add_system_node(
            node::COLOR_MATERIAL,
            AssetRenderResourcesNode::<ColorMaterial>::new(false),
        );
        self.add_node_edge(node::COLOR_MATERIAL, base::node::MAIN_PASS)
            .unwrap();

        self.add_system_node(node::SPRITE, RenderResourcesNode::<Sprite>::new(true));
        self.add_node_edge(node::SPRITE, base::node::MAIN_PASS)
            .unwrap();

        self.add_system_node(
            node::SPRITE_SHEET,
            AssetRenderResourcesNode::<TextureAtlas>::new(false),
        );

        self.add_system_node(
            node::SPRITE_SHEET_SPRITE,
            RenderResourcesNode::<TextureAtlasSprite>::new(true),
        );

        let mut pipelines = resources.get_mut::<Assets<PipelineDescriptor>>().unwrap();
        let mut shaders = resources.get_mut::<Assets<Shader>>().unwrap();
        pipelines.set_untracked(SPRITE_PIPELINE_HANDLE, build_sprite_pipeline(&mut shaders));
        pipelines.set_untracked(
            SPRITE_SHEET_PIPELINE_HANDLE,
            build_sprite_sheet_pipeline(&mut shaders),
        );
        self
    }
}
