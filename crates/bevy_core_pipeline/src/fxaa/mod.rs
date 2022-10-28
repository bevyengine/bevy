use std::borrow::Cow;

use bevy_derive::Deref;
use bevy_ecs::query::QueryItem;
use bevy_render::camera::ExtractedCamera;
use bevy_render::extract_component::{ExtractComponent, ExtractComponentPlugin};
use bevy_render::prelude::Camera;
use bevy_render::render_graph::RenderGraph;
use bevy_render::texture::{BevyDefault, CachedTexture, TextureCache};
use bevy_utils::HashMap;

use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_ecs::prelude::*;
use bevy_render::renderer::RenderDevice;
use bevy_render::view::{ExtractedView, Msaa, ViewTarget};
use bevy_render::{render_resource::*, RenderApp, RenderStage};

use bevy_reflect::TypeUuid;

mod node;

use crate::fullscreen_vertex_shader::fullscreen_shader_vertex_state;
use crate::fxaa::node::FXAANode;
use crate::{core_2d, core_3d};
#[derive(Clone)]
pub enum Quality {
    Low,
    Medium,
    High,
    Ultra,
}

impl Quality {
    pub fn get_str(&self) -> &str {
        match self {
            Quality::Low => "LOW",
            Quality::Medium => "MEDIUM",
            Quality::High => "HIGH",
            Quality::Ultra => "ULTRA",
        }
    }
}

#[derive(Component, Clone)]
pub struct FXAA {
    /// Enable render passes for FXAA.
    pub enabled: bool,

    /// The minimum amount of local contrast required to apply algorithm.
    /// Use lower settings for a sharper, faster, result.
    /// Use higher settings for a slower, smoother, result.
    pub edge_threshold: Quality,

    /// Trims the algorithm from processing darks.
    /// Use lower settings for a sharper, faster, result.
    /// Use higher settings for a slower, smoother, result.
    pub edge_threshold_min: Quality,
}

impl Default for FXAA {
    fn default() -> Self {
        FXAA {
            enabled: true,
            edge_threshold: Quality::High,
            edge_threshold_min: Quality::High,
        }
    }
}

impl FXAA {
    pub fn get_settings(&self) -> Vec<String> {
        vec![
            format!("EDGE_THRESH_{}", self.edge_threshold.get_str()),
            format!("EDGE_THRESH_MIN_{}", self.edge_threshold_min.get_str()),
        ]
    }
}

impl ExtractComponent for FXAA {
    type Query = &'static Self;
    type Filter = With<Camera>;

    fn extract_component(item: QueryItem<Self::Query>) -> Self {
        item.clone()
    }
}

const LDR_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 7112161265414793412);

const FXAA_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 4182761465141723543);

const BLIT_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 8382381532578979433);

pub const FXAA_NODE_3D: &str = "fxaa_node_3d";
pub const FXAA_NODE_2D: &str = "fxaa_node_2d";

pub struct FXAAPlugin;
impl Plugin for FXAAPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Msaa { samples: 1 }); // Disable MSAA be default

        load_internal_asset!(app, LDR_SHADER_HANDLE, "to_ldr.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, FXAA_SHADER_HANDLE, "fxaa.wgsl", Shader::from_wgsl);
        load_internal_asset!(app, BLIT_SHADER_HANDLE, "blit.wgsl", Shader::from_wgsl);

        app.add_plugin(ExtractComponentPlugin::<FXAA>::default());

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };
        render_app
            .init_resource::<FXAAPipelineBindGroup>()
            .add_system_to_stage(RenderStage::Prepare, prepare_fxaa_texture);

        {
            let fxaa_node = FXAANode::new(&mut render_app.world);
            let mut binding = render_app.world.resource_mut::<RenderGraph>();
            let graph = binding.get_sub_graph_mut(core_3d::graph::NAME).unwrap();

            graph.add_node(FXAA_NODE_3D, fxaa_node);

            graph
                .add_slot_edge(
                    graph.input_node().unwrap().id,
                    core_3d::graph::input::VIEW_ENTITY,
                    FXAA_NODE_3D,
                    FXAANode::IN_VIEW,
                )
                .unwrap();

            graph
                .add_node_edge(core_3d::graph::node::MAIN_PASS, FXAA_NODE_3D)
                .unwrap();

            graph
                .add_node_edge(FXAA_NODE_3D, core_3d::graph::node::TONEMAPPING)
                .unwrap();
        }
        {
            let fxaa_node = FXAANode::new(&mut render_app.world);
            let mut binding = render_app.world.resource_mut::<RenderGraph>();
            let graph = binding.get_sub_graph_mut(core_2d::graph::NAME).unwrap();

            graph.add_node(FXAA_NODE_2D, fxaa_node);

            graph
                .add_slot_edge(
                    graph.input_node().unwrap().id,
                    core_2d::graph::input::VIEW_ENTITY,
                    FXAA_NODE_2D,
                    FXAANode::IN_VIEW,
                )
                .unwrap();

            graph
                .add_node_edge(core_2d::graph::node::MAIN_PASS, FXAA_NODE_2D)
                .unwrap();

            graph
                .add_node_edge(FXAA_NODE_2D, core_2d::graph::node::TONEMAPPING)
                .unwrap();
        }
    }
}

#[derive(Resource, Deref)]
pub struct FXAAPipelineBindGroup(BindGroupLayout);

impl FromWorld for FXAAPipelineBindGroup {
    fn from_world(render_world: &mut World) -> Self {
        let fxaa_texture_bind_group = render_world
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

        FXAAPipelineBindGroup(fxaa_texture_bind_group)
    }
}

fn fullscreen_vertex_pipeline_descriptor(
    label: &'static str,
    bind_group_layout: &BindGroupLayout,
    shader: HandleUntyped,
    shader_defs: Vec<String>,
    entry_point: &'static str,
    format: TextureFormat,
) -> RenderPipelineDescriptor {
    RenderPipelineDescriptor {
        label: Some(label.into()),
        layout: Some(vec![bind_group_layout.clone()]),
        vertex: fullscreen_shader_vertex_state(),
        fragment: Some(FragmentState {
            shader: shader.typed(),
            shader_defs,
            entry_point: Cow::Borrowed(entry_point),
            targets: vec![Some(ColorTargetState {
                format,
                blend: None,
                write_mask: ColorWrites::ALL,
            })],
        }),
        primitive: PrimitiveState::default(),
        depth_stencil: None,
        multisample: MultisampleState::default(),
    }
}

#[derive(Component)]
pub struct FXAATexture {
    pub output: CachedTexture,
}

#[derive(Component)]
pub struct FXAAPipelines {
    pub fxaa_ldr_pipeline_id: CachedRenderPipelineId,
    pub fxaa_hdr_pipeline_id: CachedRenderPipelineId,
    pub to_ldr_pipeline_id: CachedRenderPipelineId,
    pub blit_pipeline_id: CachedRenderPipelineId,
}

pub fn prepare_fxaa_texture(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    mut pipeline_cache: ResMut<PipelineCache>,
    bind_group: Res<FXAAPipelineBindGroup>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera, &ExtractedView, &FXAA)>,
) {
    let mut output_textures = HashMap::default();

    for (entity, camera, view, fxaa) in &views {
        if let Some(physical_target_size) = camera.physical_target_size {
            let mut texture_descriptor = TextureDescriptor {
                label: None,
                size: Extent3d {
                    depth_or_array_layers: 1,
                    width: physical_target_size.x,
                    height: physical_target_size.y,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
                usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            };

            texture_descriptor.label = Some("fxaa_view_target_texture");

            let output = output_textures
                .entry(camera.target.clone())
                .or_insert_with(|| texture_cache.get(&render_device, texture_descriptor))
                .clone();

            let shader_defs = fxaa.get_settings();
            let fxaa_ldr_descriptor = fullscreen_vertex_pipeline_descriptor(
                "fxaa ldr pipeline",
                &bind_group,
                FXAA_SHADER_HANDLE,
                shader_defs,
                "fs_main",
                TextureFormat::bevy_default(),
            );

            let mut shader_defs = fxaa.get_settings();
            shader_defs.push(String::from("TONEMAP"));
            let fxaa_hdr_descriptor = fullscreen_vertex_pipeline_descriptor(
                "fxaa hdr pipeline",
                &bind_group,
                FXAA_SHADER_HANDLE,
                shader_defs,
                "fs_main",
                ViewTarget::TEXTURE_FORMAT_HDR,
            );

            let to_ldr_descriptor = fullscreen_vertex_pipeline_descriptor(
                "to ldr pipeline",
                &bind_group,
                LDR_SHADER_HANDLE,
                vec![],
                "fs_main",
                ViewTarget::TEXTURE_FORMAT_HDR,
            );

            let blit_descriptor = fullscreen_vertex_pipeline_descriptor(
                "blit pipeline",
                &bind_group,
                BLIT_SHADER_HANDLE,
                vec![],
                "fs_main",
                TextureFormat::bevy_default(),
            );

            let pipelines = FXAAPipelines {
                fxaa_ldr_pipeline_id: pipeline_cache.queue_render_pipeline(fxaa_ldr_descriptor),
                fxaa_hdr_pipeline_id: pipeline_cache.queue_render_pipeline(fxaa_hdr_descriptor),
                to_ldr_pipeline_id: pipeline_cache.queue_render_pipeline(to_ldr_descriptor),
                blit_pipeline_id: pipeline_cache.queue_render_pipeline(blit_descriptor),
            };

            commands
                .entity(entity)
                .insert(FXAATexture { output })
                .insert(pipelines);
        }
    }
}
