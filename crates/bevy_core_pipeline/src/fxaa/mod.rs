mod node;

use crate::{
    core_2d, core_3d, fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    fxaa::node::FxaaNode,
};
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_derive::Deref;
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_reflect::TypeUuid;
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    prelude::Camera,
    render_graph::RenderGraph,
    render_resource::*,
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ExtractedView, ViewTarget},
    RenderApp, RenderStage,
};

#[derive(Eq, PartialEq, Hash, Clone, Copy)]
pub enum Sensitivity {
    Low,
    Medium,
    High,
    Ultra,
    Extreme,
}

impl Sensitivity {
    pub fn get_str(&self) -> &str {
        match self {
            Sensitivity::Low => "LOW",
            Sensitivity::Medium => "MEDIUM",
            Sensitivity::High => "HIGH",
            Sensitivity::Ultra => "ULTRA",
            Sensitivity::Extreme => "EXTREME",
        }
    }
}

#[derive(Component, Clone)]
pub struct Fxaa {
    /// Enable render passes for FXAA.
    pub enabled: bool,

    /// Use lower sensitivity for a sharper, faster, result.
    /// Use higher sensitivity for a slower, smoother, result.
    /// Ultra and Turbo settings can result in significant smearing and loss of detail.

    /// The minimum amount of local contrast required to apply algorithm.
    pub edge_threshold: Sensitivity,

    /// Trims the algorithm from processing darks.
    pub edge_threshold_min: Sensitivity,
}

impl Default for Fxaa {
    fn default() -> Self {
        Fxaa {
            enabled: true,
            edge_threshold: Sensitivity::High,
            edge_threshold_min: Sensitivity::High,
        }
    }
}

impl ExtractComponent for Fxaa {
    type Query = &'static Self;
    type Filter = With<Camera>;

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}

const FXAA_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4182761465141723543);

pub const FXAA_NODE_3D: &str = "fxaa_node_3d";
pub const FXAA_NODE_2D: &str = "fxaa_node_2d";

/// Adds support for Fast Approximate Anti-Aliasing (FXAA)
pub struct FxaaPlugin;
impl Plugin for FxaaPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(app, FXAA_SHADER_HANDLE, "fxaa.wgsl", Shader::from_wgsl);

        app.add_plugin(ExtractComponentPlugin::<Fxaa>::default());

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };
        render_app
            .init_resource::<FxaaPipeline>()
            .init_resource::<SpecializedRenderPipelines<FxaaPipeline>>()
            .add_system_to_stage(RenderStage::Prepare, prepare_fxaa_pipelines);

        {
            let fxaa_node = FxaaNode::new(&mut render_app.world);
            let mut binding = render_app.world.resource_mut::<RenderGraph>();
            let graph = binding.get_sub_graph_mut(core_3d::graph::NAME).unwrap();

            graph.add_node(FXAA_NODE_3D, fxaa_node);

            graph
                .add_slot_edge(
                    graph.input_node().unwrap().id,
                    core_3d::graph::input::VIEW_ENTITY,
                    FXAA_NODE_3D,
                    FxaaNode::IN_VIEW,
                )
                .unwrap();

            graph
                .add_node_edge(core_3d::graph::node::TONEMAPPING, FXAA_NODE_3D)
                .unwrap();
            graph
                .add_node_edge(
                    FXAA_NODE_3D,
                    core_3d::graph::node::END_MAIN_PASS_POST_PROCESSING,
                )
                .unwrap();
        }
        {
            let fxaa_node = FxaaNode::new(&mut render_app.world);
            let mut binding = render_app.world.resource_mut::<RenderGraph>();
            let graph = binding.get_sub_graph_mut(core_2d::graph::NAME).unwrap();

            graph.add_node(FXAA_NODE_2D, fxaa_node);

            graph
                .add_slot_edge(
                    graph.input_node().unwrap().id,
                    core_2d::graph::input::VIEW_ENTITY,
                    FXAA_NODE_2D,
                    FxaaNode::IN_VIEW,
                )
                .unwrap();

            graph
                .add_node_edge(core_2d::graph::node::TONEMAPPING, FXAA_NODE_2D)
                .unwrap();
            graph
                .add_node_edge(
                    FXAA_NODE_2D,
                    core_2d::graph::node::END_MAIN_PASS_POST_PROCESSING,
                )
                .unwrap();
        }
    }
}

#[derive(Resource, Deref)]
pub struct FxaaPipeline {
    texture_bind_group: BindGroupLayout,
}

impl FromWorld for FxaaPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let texture_bind_group = render_world
            .resource::<RenderDevice>()
            .create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("fxaa_texture_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: true },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 1,
                        visibility: ShaderStages::FRAGMENT,
                        ty: BindingType::Sampler(SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
            });

        FxaaPipeline { texture_bind_group }
    }
}

#[derive(Component)]
pub struct CameraFxaaPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct FxaaPipelineKey {
    edge_threshold: Sensitivity,
    edge_threshold_min: Sensitivity,
    texture_format: TextureFormat,
}

impl SpecializedRenderPipeline for FxaaPipeline {
    type Key = FxaaPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("fxaa".into()),
            layout: Some(vec![self.texture_bind_group.clone()]),
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: FXAA_SHADER_HANDLE.typed(),
                shader_defs: vec![
                    format!("EDGE_THRESH_{}", key.edge_threshold.get_str()),
                    format!("EDGE_THRESH_MIN_{}", key.edge_threshold_min.get_str()),
                ],
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
        }
    }
}

pub fn prepare_fxaa_pipelines(
    mut commands: Commands,
    mut pipeline_cache: ResMut<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<FxaaPipeline>>,
    fxaa_pipeline: Res<FxaaPipeline>,
    views: Query<(Entity, &ExtractedView, &Fxaa)>,
) {
    for (entity, view, fxaa) in &views {
        if !fxaa.enabled {
            continue;
        }
        let pipeline_id = pipelines.specialize(
            &mut pipeline_cache,
            &fxaa_pipeline,
            FxaaPipelineKey {
                edge_threshold: fxaa.edge_threshold,
                edge_threshold_min: fxaa.edge_threshold_min,
                texture_format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
            },
        );

        commands
            .entity(entity)
            .insert(CameraFxaaPipeline { pipeline_id });
    }
}
