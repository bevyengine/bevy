use crate::ScreenSpaceAmbientOcclusionTextures;
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, HandleUntyped};
use bevy_core_pipeline::{
    clear_color::ClearColorConfig,
    core_3d::{self, CORE_3D_DEPTH_FORMAT},
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
    prelude::{Camera3d, ClearColor},
    prepass::{DeferredPrepass, ViewPrepassTextures},
    tonemapping::{get_lut_bindings, DebandDither, Tonemapping, TonemappingLuts},
};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    extract_resource::{ExtractResource, ExtractResourcePlugin},
    globals::GlobalsBuffer,
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::{BindGroupDescriptor, Operations, PipelineCache, RenderPassDescriptor},
    renderer::RenderContext,
    texture::{FallbackImageCubemap, FallbackImageMsaa, Image},
    view::{Msaa, ViewDepthTexture, ViewTarget, ViewUniformOffset, ViewUniforms},
    Render, RenderSet,
};

use bevy_reflect::{Reflect, TypeUuid};
use bevy_render::{
    render_graph::RenderGraphApp, render_resource::*, renderer::RenderDevice, texture::BevyDefault,
    view::ExtractedView, RenderApp,
};

use crate::{
    environment_map, mesh_view_layout_entries, prepass, EnvironmentMapLight, FogMeta,
    GlobalLightMeta, LightMeta, MeshPipelineKey, ShadowSamplers, ViewClusterBindings,
    ViewFogUniformOffset, ViewLightsUniformOffset, ViewShadowBindings,
    CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT, MAX_CASCADES_PER_LIGHT, MAX_DIRECTIONAL_LIGHTS,
};

pub struct PBRDeferredLightingPlugin;

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

#[derive(Resource)]
pub struct BypassPBRDeferredLightingPlugin;

impl Plugin for PBRDeferredLightingPlugin {
    fn build(&self, app: &mut App) {
        if app
            .world
            .contains_resource::<BypassPBRDeferredLightingPlugin>()
        {
            return;
        }

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
                (
                    queue_deferred_lighting_bind_groups.in_set(RenderSet::Queue),
                    prepare_deferred_lighting_pipelines.in_set(RenderSet::Prepare),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<DeferredLightingNode>>(
                core_3d::graph::NAME,
                DEFERRED_LIGHTING_NODE_NAME,
            )
            .add_render_graph_edges(
                core_3d::graph::NAME,
                &[
                    core_3d::graph::node::START_MAIN_PASS,
                    DEFERRED_LIGHTING_NODE_NAME,
                    core_3d::graph::node::MAIN_OPAQUE_PASS,
                ],
            );
    }

    fn finish(&self, app: &mut App) {
        if app
            .world
            .contains_resource::<BypassPBRDeferredLightingPlugin>()
        {
            return;
        }
        let Ok(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app.init_resource::<DeferredLightingLayout>();
    }
}

pub const DEFERRED_LIGHTING_NODE_NAME: &str = "deferred_lighting";

#[derive(Default)]
struct DeferredLightingNode;

impl ViewNode for DeferredLightingNode {
    type ViewQuery = (
        &'static ViewUniformOffset,
        &'static ViewLightsUniformOffset,
        &'static ViewFogUniformOffset,
        &'static DeferredLightingBindGroup,
        &'static ViewTarget,
        &'static ViewDepthTexture,
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
            view_depth_texture,
            camera_3d,
            deferred_lighting_pipeline,
        ): QueryItem<Self::ViewQuery>,
        world: &World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let stencil_reference = world.resource::<PBRDeferredLightingStencilReference>();

        let Some(pipeline) = pipeline_cache.get_render_pipeline(deferred_lighting_pipeline.pipeline_id) else {
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
                view: &view_depth_texture.view,
                depth_ops: None,
                stencil_ops: Some(Operations {
                    load: LoadOp::Load,
                    store: false,
                }),
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
        render_pass.set_stencil_reference(stencil_reference.0);
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
        shader_defs.push("WEBGL".into());

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
            vertex: fullscreen_shader_vertex_state(),
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
                format: CORE_3D_DEPTH_FORMAT,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: StencilFaceState {
                        compare: CompareFunction::Equal,
                        fail_op: StencilOperation::Keep,
                        depth_fail_op: StencilOperation::Keep,
                        pass_op: StencilOperation::Keep,
                    },
                    back: StencilFaceState {
                        compare: CompareFunction::Equal,
                        fail_op: StencilOperation::Keep,
                        depth_fail_op: StencilOperation::Keep,
                        pass_op: StencilOperation::Keep,
                    },
                    read_mask: u32::MAX,
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
        let render_device = world.resource::<RenderDevice>();

        let clustered_forward_buffer_binding_type = render_device
            .get_supported_read_only_binding_type(CLUSTERED_FORWARD_STORAGE_BUFFER_COUNT);

        let bind_group_layout =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("deferred_lighting_bind_group_layout"),
                entries: &mesh_view_layout_entries(clustered_forward_buffer_binding_type, false),
            });

        Self { bind_group_layout }
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
        ),
        With<DeferredPrepass>,
    >,
    images: Res<RenderAssets<Image>>,
) {
    for (entity, view, tonemapping, dither, environment_map) in &views {
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

#[derive(Component)]
pub struct DeferredLightingBindGroup {
    pub value: BindGroup,
}

#[allow(clippy::too_many_arguments)]
pub fn queue_deferred_lighting_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    shadow_samplers: Res<ShadowSamplers>,
    light_meta: Res<LightMeta>,
    global_light_meta: Res<GlobalLightMeta>,
    fog_meta: Res<FogMeta>,
    view_uniforms: Res<ViewUniforms>,
    views: Query<
        (
            Entity,
            &ViewShadowBindings,
            &ViewClusterBindings,
            Option<&ScreenSpaceAmbientOcclusionTextures>,
            Option<&ViewPrepassTextures>,
            Option<&EnvironmentMapLight>,
            &Tonemapping,
        ),
        With<DeferredPrepass>,
    >,
    images: Res<RenderAssets<Image>>,
    mut fallback_images: FallbackImageMsaa,
    fallback_cubemap: Res<FallbackImageCubemap>,
    msaa: Res<Msaa>,
    globals_buffer: Res<GlobalsBuffer>,
    tonemapping_luts: Res<TonemappingLuts>,
    differed_lighting_layout: Res<DeferredLightingLayout>,
) {
    if let (
        Some(view_binding),
        Some(light_binding),
        Some(point_light_binding),
        Some(globals),
        Some(fog_binding),
    ) = (
        view_uniforms.uniforms.binding(),
        light_meta.view_gpu_lights.binding(),
        global_light_meta.gpu_point_lights.binding(),
        globals_buffer.buffer.binding(),
        fog_meta.gpu_fogs.binding(),
    ) {
        for (
            entity,
            view_shadow_bindings,
            view_cluster_bindings,
            ssao_textures,
            prepass_textures,
            environment_map,
            tonemapping,
        ) in &views
        {
            let fallback_ssao = fallback_images
                .image_for_samplecount(1, TextureFormat::Rgba8Unorm)
                .texture_view
                .clone();

            let mut entries = vec![
                BindGroupEntry {
                    binding: 0,
                    resource: view_binding.clone(),
                },
                BindGroupEntry {
                    binding: 1,
                    resource: light_binding.clone(),
                },
                BindGroupEntry {
                    binding: 2,
                    resource: BindingResource::TextureView(
                        &view_shadow_bindings.point_light_depth_texture_view,
                    ),
                },
                BindGroupEntry {
                    binding: 3,
                    resource: BindingResource::Sampler(&shadow_samplers.point_light_sampler),
                },
                BindGroupEntry {
                    binding: 4,
                    resource: BindingResource::TextureView(
                        &view_shadow_bindings.directional_light_depth_texture_view,
                    ),
                },
                BindGroupEntry {
                    binding: 5,
                    resource: BindingResource::Sampler(&shadow_samplers.directional_light_sampler),
                },
                BindGroupEntry {
                    binding: 6,
                    resource: point_light_binding.clone(),
                },
                BindGroupEntry {
                    binding: 7,
                    resource: view_cluster_bindings.light_index_lists_binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 8,
                    resource: view_cluster_bindings.offsets_and_counts_binding().unwrap(),
                },
                BindGroupEntry {
                    binding: 9,
                    resource: globals.clone(),
                },
                BindGroupEntry {
                    binding: 10,
                    resource: fog_binding.clone(),
                },
                BindGroupEntry {
                    binding: 11,
                    resource: BindingResource::TextureView(
                        ssao_textures
                            .map(|t| &t.screen_space_ambient_occlusion_texture.default_view)
                            .unwrap_or(&fallback_ssao),
                    ),
                },
            ];

            let env_map = environment_map::get_bindings(
                environment_map,
                &images,
                &fallback_cubemap,
                [12, 13, 14],
            );
            entries.extend_from_slice(&env_map);

            let tonemapping_luts =
                get_lut_bindings(&images, &tonemapping_luts, tonemapping, [15, 16]);
            entries.extend_from_slice(&tonemapping_luts);

            let prepass_bindings =
                prepass::get_bindings(prepass_textures, &mut fallback_images, &msaa);
            // When using WebGL, we can't have a depth texture with multisampling
            if cfg!(any(not(feature = "webgl"), not(target_arch = "wasm32")))
                || (cfg!(all(feature = "webgl", target_arch = "wasm32")) && msaa.samples() == 1)
            {
                entries.extend_from_slice(&prepass_bindings.get_entries([17, 18, 19, 20]));
            }

            let view_bind_group = render_device.create_bind_group(&BindGroupDescriptor {
                entries: &entries,
                label: Some("deferred_mesh_view_bind_group"),
                layout: &differed_lighting_layout.bind_group_layout,
            });

            commands.entity(entity).insert(DeferredLightingBindGroup {
                value: view_bind_group,
            });
        }
    }
}
