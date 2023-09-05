use crate::{MeshPipeline, MeshViewBindGroup, ScreenSpaceAmbientOcclusionSettings};
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_core_pipeline::{
    clear_color::ClearColorConfig,
    core_3d,
    deferred::{
        copy_lighting_id::DeferredLightingIdDepthTexture, DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT,
    },
    prelude::{Camera3d, ClearColor},
    prepass::DeferredPrepass,
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::{Operations, PipelineCache, RenderPassDescriptor},
    renderer::RenderContext,
    texture::Image,
    view::{ViewTarget, ViewUniformOffset},
    Render, RenderSet,
};

use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    render_graph::RenderGraphApp, render_resource::*, texture::BevyDefault, view::ExtractedView,
    RenderApp,
};

use crate::{
    EnvironmentMapLight, MeshPipelineKey, ViewFogUniformOffset, ViewLightsUniformOffset,
    MAX_CASCADES_PER_LIGHT, MAX_DIRECTIONAL_LIGHTS,
};

pub struct PbrDeferredLightingPlugin;

pub const DEFERRED_LIGHTING_SHADER_HANDLE: HandleUntyped =
    HandleUntyped::weak_from_u64(Shader::TYPE_UUID, 2708011359337029741);

pub const DEFAULT_PBR_DEFERRED_LIGHTING_STENCIL_REFERENCE: u32 = 1;

#[derive(Resource, Clone, Debug, ExtractResource, Reflect)]
pub struct PBRDeferredLightingStencilReference(pub u32);

impl Default for PBRDeferredLightingStencilReference {
    fn default() -> Self {
        Self(DEFAULT_PBR_DEFERRED_LIGHTING_STENCIL_REFERENCE)
    }
}

impl Plugin for PbrDeferredLightingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PBRDeferredLightingStencilReference>()
            .register_type::<PBRDeferredLightingStencilReference>()
            .add_plugins(ExtractResourcePlugin::<PBRDeferredLightingStencilReference>::default());

        load_internal_asset!(
            app,
            DEFERRED_LIGHTING_SHADER_HANDLE,
            "deferred_lighting.wgsl",
            Shader::from_wgsl
        );

        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<DeferredLightingLayout>>()
            .add_systems(
                Render,
                (prepare_deferred_lighting_pipelines.in_set(RenderSet::Prepare),),
            )
            .add_render_graph_node::<ViewNodeRunner<DeferredLightingNode>>(
                core_3d::graph::NAME,
                DEFERRED_LIGHTING_PASS,
            )
            .add_render_graph_edges(
                core_3d::graph::NAME,
                &[
                    core_3d::graph::node::START_MAIN_PASS,
                    DEFERRED_LIGHTING_PASS,
                    core_3d::graph::node::MAIN_OPAQUE_PASS,
                ],
            );
    }

    fn finish(&self, app: &mut App) {
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<DeferredLightingLayout>();
    }
}

pub const DEFERRED_LIGHTING_PASS: &str = "deferred_lighting";

#[derive(Default)]
pub struct DeferredLightingNode;

impl ViewNode for DeferredLightingNode {
    type ViewQuery = (
        &'static ViewUniformOffset,
        &'static ViewLightsUniformOffset,
        &'static ViewFogUniformOffset,
        &'static MeshViewBindGroup,
        &'static ViewTarget,
        &'static DeferredLightingIdDepthTexture,
        &'static Camera3d,
        &'static DeferredLightingPipeline,
    );

    fn run(
        &self,
        _graph_context: &mut RenderGraphContext,
        render_context: &mut RenderContext,
        (
            view_uniform_offset,
            view_lights_offset,
            view_fog_offset,
            mesh_view_bind_group,
            target,
            deferred_lighting_id_depth_texture,
            camera_3d,
            deferred_lighting_pipeline,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();

        let Some(pipeline) =
            pipeline_cache.get_render_pipeline(deferred_lighting_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let mut render_pass = render_context.begin_tracked_render_pass(RenderPassDescriptor {
            label: Some("deferred_lighting_pass"),
            color_attachments: &[Some(target.get_color_attachment(Operations {
                load: match camera_3d.clear_color {
                    ClearColorConfig::Default => {
                        LoadOp::Clear(world.resource::<ClearColor>().0.into())
                    }
                    ClearColorConfig::Custom(color) => LoadOp::Clear(color.into()),
                    ClearColorConfig::None => LoadOp::Load,
                },
                store: true,
            }))],
            depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
                view: &deferred_lighting_id_depth_texture.texture.default_view,
                depth_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: false,
                }),
                stencil_ops: None,
            }),
        });

        render_pass.set_render_pipeline(pipeline);
        render_pass.set_bind_group(
            0,
            &mesh_view_bind_group.value,
            &[
                view_uniform_offset.offset,
                view_lights_offset.offset,
                view_fog_offset.offset,
            ],
        );
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
pub struct DeferredLightingLayout {
    bind_group_layout: BindGroupLayout,
}

#[derive(Component)]
pub struct DeferredLightingPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

impl SpecializedRenderPipeline for DeferredLightingLayout {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();

        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
        shader_defs.push("WEBGL2".into());

        if key.contains(MeshPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());

            let method = key.intersection(MeshPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == MeshPipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED ".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_AGX {
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            }

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.contains(MeshPipelineKey::DEBAND_DITHER) {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        if key.contains(MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION) {
            shader_defs.push("SCREEN_SPACE_AMBIENT_OCCLUSION".into());
        }

        if key.contains(MeshPipelineKey::ENVIRONMENT_MAP) {
            shader_defs.push("ENVIRONMENT_MAP".into());
        }

        shader_defs.push(ShaderDefVal::UInt(
            "MAX_DIRECTIONAL_LIGHTS".to_string(),
            MAX_DIRECTIONAL_LIGHTS as u32,
        ));
        shader_defs.push(ShaderDefVal::UInt(
            "MAX_CASCADES_PER_LIGHT".to_string(),
            MAX_CASCADES_PER_LIGHT as u32,
        ));

        RenderPipelineDescriptor {
            label: Some("deferred_lighting_pipeline".into()),
            layout: vec![self.bind_group_layout.clone()],
            vertex: VertexState {
                shader: DEFERRED_LIGHTING_SHADER_HANDLE.typed(),
                shader_defs: shader_defs.clone(),
                entry_point: "vertex".into(),
                buffers: Vec::new(),
            },
            fragment: Some(FragmentState {
                shader: DEFERRED_LIGHTING_SHADER_HANDLE.typed(),
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.contains(MeshPipelineKey::HDR) {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: Some(DepthStencilState {
                format: DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Equal,
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
            multisample: MultisampleState::default(),
            push_constant_ranges: vec![],
        }
    }
}

impl FromWorld for DeferredLightingLayout {
    fn from_world(world: &mut World) -> Self {
        Self {
            bind_group_layout: world.resource::<MeshPipeline>().view_layout.clone(),
        }
    }
}

pub fn prepare_deferred_lighting_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<DeferredLightingLayout>>,
    differed_lighting_layout: Res<DeferredLightingLayout>,
    views: Query<
        (
            Entity,
            &ExtractedView,
            Option<&Tonemapping>,
            Option<&DebandDither>,
            Option<&EnvironmentMapLight>,
            Option<&ScreenSpaceAmbientOcclusionSettings>,
        ),
        With<DeferredPrepass>,
    >,
    images: Res<RenderAssets<Image>>,
) {
    for (entity, view, tonemapping, dither, environment_map, ssao) in &views {
        let mut view_key = MeshPipelineKey::from_hdr(view.hdr);

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key |= MeshPipelineKey::TONEMAP_IN_SHADER;
                view_key |= match tonemapping {
                    Tonemapping::None => MeshPipelineKey::TONEMAP_METHOD_NONE,
                    Tonemapping::Reinhard => MeshPipelineKey::TONEMAP_METHOD_REINHARD,
                    Tonemapping::ReinhardLuminance => {
                        MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE
                    }
                    Tonemapping::AcesFitted => MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED,
                    Tonemapping::AgX => MeshPipelineKey::TONEMAP_METHOD_AGX,
                    Tonemapping::SomewhatBoringDisplayTransform => {
                        MeshPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM
                    }
                    Tonemapping::TonyMcMapface => MeshPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE,
                    Tonemapping::BlenderFilmic => MeshPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC,
                };
            }
            if let Some(DebandDither::Enabled) = dither {
                view_key |= MeshPipelineKey::DEBAND_DITHER;
            }
        }

        if ssao.is_some() {
            view_key |= MeshPipelineKey::SCREEN_SPACE_AMBIENT_OCCLUSION;
        }

        let environment_map_loaded = match environment_map {
            Some(environment_map) => environment_map.is_loaded(&images),
            None => false,
        };
        if environment_map_loaded {
            view_key |= MeshPipelineKey::ENVIRONMENT_MAP;
        }

        let pipeline_id =
            pipelines.specialize(&pipeline_cache, &differed_lighting_layout, view_key);

        commands
            .entity(entity)
            .insert(DeferredLightingPipeline { pipeline_id });
    }
}
