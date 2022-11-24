// Ground Truth-based Ambient Occlusion (GTAO)
// Paper: https://www.activision.com/cdn/research/Practical_Real_Time_Strategies_for_Accurate_Indirect_Occlusion_NEW%20VERSION_COLOR.pdf
// Presentation: https://blog.selfshadow.com/publications/s2016-shading-course/activision/s2016_pbs_activision_occlusion.pdf

// Source code heavily based on XeGTAO v1.30 from Intel
// https://github.com/GameTechDev/XeGTAO/blob/0d177ce06bfa642f64d8af4de1197ad1bcb862d4/Source/Rendering/Shaders/XeGTAO.hlsli

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_core_pipeline::{prelude::Camera3d, prepass::PrepassSettings};
use bevy_ecs::{
    prelude::{Component, Entity},
    query::{QueryState, With},
    system::{Commands, Query, Resource},
    world::{FromWorld, World},
};
use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    camera::ExtractedCamera,
    prelude::Camera,
    render_graph::{Node, NodeRunError, RenderGraph, RenderGraphContext, SlotInfo, SlotType},
    render_resource::{
        AddressMode, BindGroupLayout, BindGroupLayoutDescriptor, BindGroupLayoutEntry, BindingType,
        BufferBindingType, CachedComputePipelineId, ComputePipelineDescriptor, FilterMode,
        PipelineCache, Sampler, SamplerBindingType, SamplerDescriptor, Shader, ShaderStages,
        ShaderType, StorageTextureAccess, TextureSampleType, TextureViewDimension,
    },
    renderer::{RenderContext, RenderDevice},
    view::{ViewTarget, ViewUniform},
    Extract, RenderApp, RenderStage,
};

use crate::PREPASS_DEPTH_FORMAT;

pub mod draw_3d_graph {
    pub mod node {
        /// Label for the ambient occlusion render node.
        pub const AMBIENT_OCCLUSION: &str = "ambient_occlusion";
    }
}

const PREFILTER_DEPTH_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 549599446926908);

// TODO: Support MSAA

pub struct AmbientOcclusionPlugin;

impl Plugin for AmbientOcclusionPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            PREFILTER_DEPTH_SHADER_HANDLE,
            "prefilter_depth.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<AmbientOcclusionSettings>();

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };

        render_app
            .init_resource::<AmbientOcclusionPipelines>()
            //     .init_resource::<BloomUniforms>()
            .add_system_to_stage(RenderStage::Extract, extract_ao_settings);
        //     .add_system_to_stage(RenderStage::Prepare, prepare_bloom_textures)
        //     .add_system_to_stage(RenderStage::Prepare, prepare_bloom_uniforms)
        //     .add_system_to_stage(RenderStage::Queue, queue_bloom_bind_groups);

        let bloom_node = AmbientOcclusionNode::new(&mut render_app.world);
        let mut graph = render_app.world.resource_mut::<RenderGraph>();
        let draw_3d_graph = graph
            .get_sub_graph_mut(bevy_core_pipeline::core_3d::graph::NAME)
            .unwrap();
        draw_3d_graph.add_node(draw_3d_graph::node::AMBIENT_OCCLUSION, bloom_node);
        draw_3d_graph
            .add_slot_edge(
                draw_3d_graph.input_node().unwrap().id,
                bevy_core_pipeline::core_3d::graph::input::VIEW_ENTITY,
                draw_3d_graph::node::AMBIENT_OCCLUSION,
                AmbientOcclusionNode::IN_VIEW,
            )
            .unwrap();
        // PREPASS -> AMBIENT_OCCLUSION -> MAIN_PASS
        draw_3d_graph
            .add_node_edge(
                bevy_core_pipeline::core_3d::graph::node::PREPASS,
                draw_3d_graph::node::AMBIENT_OCCLUSION,
            )
            .unwrap();
        draw_3d_graph
            .add_node_edge(
                draw_3d_graph::node::AMBIENT_OCCLUSION,
                bevy_core_pipeline::core_3d::graph::node::MAIN_PASS,
            )
            .unwrap();
    }
}

#[derive(Component, Reflect, Clone)]
pub struct AmbientOcclusionSettings {
    effect_radius: f32,
    effect_falloff_range: f32,
}

impl Default for AmbientOcclusionSettings {
    fn default() -> Self {
        // TODO: Document defaults
        Self {
            effect_radius: 0.5,
            effect_falloff_range: 0.615,
        }
    }
}

struct AmbientOcclusionNode {
    view_query: QueryState<(
        &'static ExtractedCamera,
        &'static ViewTarget,
        // &'static BloomTextures,
        // &'static BloomBindGroups,
        // &'static BloomUniformIndex,
    )>,
}

impl AmbientOcclusionNode {
    const IN_VIEW: &'static str = "view";

    fn new(world: &mut World) -> Self {
        Self {
            view_query: QueryState::new(world),
        }
    }
}

impl Node for AmbientOcclusionNode {
    fn input(&self) -> Vec<SlotInfo> {
        vec![SlotInfo::new(Self::IN_VIEW, SlotType::Entity)]
    }

    fn update(&mut self, world: &mut World) {
        self.view_query.update_archetypes(world);
    }

    fn run(
        &self,
        graph: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        world: &World,
    ) -> Result<(), NodeRunError> {
        #[cfg(feature = "trace")]
        let _ao_span = info_span!("ambient_occlusion").entered();

        todo!()
    }
}

#[derive(Resource)]
struct AmbientOcclusionPipelines {
    prefilter_depth_pipeline: CachedComputePipelineId,
    prefilter_depth_bind_group_layout: BindGroupLayout,
    point_clamp_sampler: Sampler,
}

impl FromWorld for AmbientOcclusionPipelines {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let point_clamp_sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: FilterMode::Nearest,
            mag_filter: FilterMode::Nearest,
            mipmap_filter: FilterMode::Nearest,
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            ..Default::default()
        });

        let mip_texture_entry = BindGroupLayoutEntry {
            binding: 1,
            visibility: ShaderStages::COMPUTE,
            ty: BindingType::StorageTexture {
                access: StorageTextureAccess::WriteOnly,
                format: PREPASS_DEPTH_FORMAT,
                view_dimension: TextureViewDimension::D2,
            },
            count: None,
        };
        let prefilter_depth_bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("prefilter_depth_bind_group_layout"),
                entries: &[
                    BindGroupLayoutEntry {
                        binding: 0,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Texture {
                            sample_type: TextureSampleType::Float { filterable: false },
                            view_dimension: TextureViewDimension::D2,
                            multisampled: false,
                        },
                        count: None,
                    },
                    mip_texture_entry,
                    BindGroupLayoutEntry {
                        binding: 2,
                        ..mip_texture_entry
                    },
                    BindGroupLayoutEntry {
                        binding: 3,
                        ..mip_texture_entry
                    },
                    BindGroupLayoutEntry {
                        binding: 4,
                        ..mip_texture_entry
                    },
                    BindGroupLayoutEntry {
                        binding: 5,
                        ..mip_texture_entry
                    },
                    BindGroupLayoutEntry {
                        binding: 6,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Sampler(SamplerBindingType::NonFiltering),
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: todo!(),
                            // min_binding_size: Some(AmbientOcclusionUniform::min_size()),
                        },
                        count: None,
                    },
                    BindGroupLayoutEntry {
                        binding: 8,
                        visibility: ShaderStages::COMPUTE,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(ViewUniform::min_size()),
                        },
                        count: None,
                    },
                ],
            });

        let mut pipeline_cache = world.resource_mut::<PipelineCache>();

        let prefilter_depth_pipeline =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("prefilter_depth_pipeline".into()),
                layout: Some(vec![prefilter_depth_bind_group_layout.clone()]),
                shader: PREFILTER_DEPTH_SHADER_HANDLE.typed(),
                shader_defs: vec![],
                entry_point: "prefilter_depth".into(),
            });

        Self {
            prefilter_depth_pipeline,
            prefilter_depth_bind_group_layout,
            point_clamp_sampler,
        }
    }
}

fn extract_ao_settings(
    mut commands: Commands,
    cameras: Extract<
        Query<(Entity, &Camera, &AmbientOcclusionSettings, &PrepassSettings), With<Camera3d>>,
    >,
) {
    for (entity, camera, ao_settings, prepass_settings) in &cameras {
        if camera.is_active && prepass_settings.output_depth && prepass_settings.output_normals {
            commands.get_or_spawn(entity).insert(ao_settings.clone());
        }
    }
}
