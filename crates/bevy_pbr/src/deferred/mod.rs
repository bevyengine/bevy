use crate::{
    irradiance_volume::IRRADIANCE_VOLUMES_ARE_USABLE, MeshPipeline, MeshPipelineKey,
    MeshPipelineSystems, MeshViewBindGroup, ViewKeyCache, TONEMAPPING_LUT_SAMPLER_BINDING_INDEX,
    TONEMAPPING_LUT_TEXTURE_BINDING_INDEX,
};
use bevy_app::prelude::*;
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_core_pipeline::{
    core_3d::main_opaque_pass_3d,
    deferred::{
        copy_lighting_id::DeferredLightingIdDepthTexture, DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT,
    },
    prepass::DeferredPrepass,
    schedule::{Core3d, Core3dSystems},
};
use bevy_ecs::prelude::*;
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    extract_component::{
        ComponentUniforms, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin,
    },
    render_resource::{binding_types::uniform_buffer, *},
    renderer::{RenderContext, ViewQuery},
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderSystems,
};
use bevy_render::{GpuResourceAppExt, RenderStartup};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;

pub struct DeferredPbrLightingPlugin;

pub const DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID: u8 = 1;

/// Component with a `depth_id` for specifying which corresponding materials should be rendered by this specific PBR deferred lighting pass.
///
/// Will be automatically added to entities with the [`DeferredPrepass`] component that don't already have a [`PbrDeferredLightingDepthId`].
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
#[extract_app(RenderApp)]
pub struct PbrDeferredLightingDepthId {
    depth_id: u32,

    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _webgl2_padding_0: f32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _webgl2_padding_1: f32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
    _webgl2_padding_2: f32,
}

impl PbrDeferredLightingDepthId {
    pub fn new(value: u8) -> PbrDeferredLightingDepthId {
        PbrDeferredLightingDepthId {
            depth_id: value as u32,

            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            _webgl2_padding_0: 0.0,
            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            _webgl2_padding_1: 0.0,
            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            _webgl2_padding_2: 0.0,
        }
    }

    pub fn set(&mut self, value: u8) {
        self.depth_id = value as u32;
    }

    pub fn get(&self) -> u8 {
        self.depth_id as u8
    }
}

impl Default for PbrDeferredLightingDepthId {
    fn default() -> Self {
        PbrDeferredLightingDepthId {
            depth_id: DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID as u32,

            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            _webgl2_padding_0: 0.0,
            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            _webgl2_padding_1: 0.0,
            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            _webgl2_padding_2: 0.0,
        }
    }
}

impl Plugin for DeferredPbrLightingPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            ExtractComponentPlugin::<PbrDeferredLightingDepthId>::default(),
            UniformComponentPlugin::<PbrDeferredLightingDepthId>::default(),
        ))
        .add_systems(PostUpdate, insert_deferred_lighting_pass_id_component);

        embedded_asset!(app, "deferred_lighting.wesl");

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_gpu_resource::<SpecializedRenderPipelines<DeferredLightingLayout>>()
            .add_systems(
                RenderStartup,
                init_deferred_lighting_layout.after(MeshPipelineSystems),
            )
            .add_systems(
                Render,
                prepare_deferred_lighting_pipelines.in_set(RenderSystems::Prepare),
            )
            .add_systems(
                Core3d,
                deferred_lighting
                    .before(main_opaque_pass_3d)
                    .in_set(Core3dSystems::MainPass),
            );
    }
}

pub fn deferred_lighting(
    view: ViewQuery<(
        &MeshViewBindGroup,
        &ViewTarget,
        &DeferredLightingIdDepthTexture,
        &DeferredLightingPipeline,
    )>,
    pipeline_cache: Res<PipelineCache>,
    deferred_lighting_layout: Res<DeferredLightingLayout>,
    deferred_lighting_pass_id: Res<ComponentUniforms<PbrDeferredLightingDepthId>>,
    mut ctx: RenderContext,
) {
    let (
        mesh_view_bind_group,
        target,
        deferred_lighting_id_depth_texture,
        deferred_lighting_pipeline,
    ) = view.into_inner();

    let Some(pipeline) = pipeline_cache.get_render_pipeline(deferred_lighting_pipeline.pipeline_id)
    else {
        return;
    };

    let Some(deferred_lighting_pass_id_binding) = deferred_lighting_pass_id.uniforms().binding()
    else {
        return;
    };

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(ctx.command_encoder(), "deferred_lighting");

    let bind_group_2 = ctx.render_device().create_bind_group(
        "deferred_lighting_layout_group_2",
        &pipeline_cache.get_bind_group_layout(&deferred_lighting_layout.bind_group_layout_2),
        &BindGroupEntries::single(deferred_lighting_pass_id_binding),
    );

    let mut render_pass = ctx.begin_tracked_render_pass(RenderPassDescriptor {
        label: Some("deferred_lighting"),
        color_attachments: &[Some(target.get_color_attachment())],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &deferred_lighting_id_depth_texture.texture.default_view,
            depth_ops: Some(Operations {
                load: LoadOp::Load,
                store: StoreOp::Discard,
            }),
            stencil_ops: None,
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });

    render_pass.set_render_pipeline(pipeline);

    render_pass.set_bind_group(
        0,
        &mesh_view_bind_group.main,
        &mesh_view_bind_group.main_offsets,
    );
    render_pass.set_bind_group(1, &mesh_view_bind_group.binding_array, &[]);
    render_pass.set_bind_group(2, &bind_group_2, &[]);
    render_pass.draw(0..3, 0..1);
    drop(render_pass);
    time_span.end(ctx.command_encoder());
}

#[derive(Resource)]
pub struct DeferredLightingLayout {
    mesh_pipeline: MeshPipeline,
    bind_group_layout_2: BindGroupLayoutDescriptor,
    deferred_lighting_shader: Handle<Shader>,
}

#[derive(Component)]
pub struct DeferredLightingPipeline {
    pub pipeline_id: CachedRenderPipelineId,
}

impl SpecializedRenderPipeline for DeferredLightingLayout {
    type Key = MeshPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();

        // Let the shader code know that it's running in a deferred pipeline.
        shader_defs.push("DEFERRED_LIGHTING_PIPELINE".into());

        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        shader_defs.push("WEBGL2".into());

        if key.contains(MeshPipelineKey::TONEMAP_IN_SHADER) {
            shader_defs.push("TONEMAP_IN_SHADER".into());
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_TEXTURE_BINDING_INDEX".into(),
                TONEMAPPING_LUT_TEXTURE_BINDING_INDEX,
            ));
            shader_defs.push(ShaderDefVal::UInt(
                "TONEMAPPING_LUT_SAMPLER_BINDING_INDEX".into(),
                TONEMAPPING_LUT_SAMPLER_BINDING_INDEX,
            ));

            let method = key.intersection(MeshPipelineKey::TONEMAP_METHOD_RESERVED_BITS);

            if method == MeshPipelineKey::TONEMAP_METHOD_NONE {
                shader_defs.push("TONEMAP_METHOD_NONE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD {
                shader_defs.push("TONEMAP_METHOD_REINHARD".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_REINHARD_LUMINANCE {
                shader_defs.push("TONEMAP_METHOD_REINHARD_LUMINANCE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_ACES_FITTED {
                shader_defs.push("TONEMAP_METHOD_ACES_FITTED".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_AGX {
                shader_defs.push("TONEMAP_METHOD_AGX".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM {
                shader_defs.push("TONEMAP_METHOD_SOMEWHAT_BORING_DISPLAY_TRANSFORM".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_BLENDER_FILMIC {
                shader_defs.push("TONEMAP_METHOD_BLENDER_FILMIC".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_TONY_MC_MAPFACE {
                shader_defs.push("TONEMAP_METHOD_TONY_MC_MAPFACE".into());
            } else if method == MeshPipelineKey::TONEMAP_METHOD_PBR_NEUTRAL {
                shader_defs.push("TONEMAP_METHOD_PBR_NEUTRAL".into());
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

        if key.contains(MeshPipelineKey::IRRADIANCE_VOLUME) && IRRADIANCE_VOLUMES_ARE_USABLE {
            shader_defs.push("IRRADIANCE_VOLUME".into());
        }

        if key.contains(MeshPipelineKey::NORMAL_PREPASS) {
            shader_defs.push("NORMAL_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::DEPTH_PREPASS) {
            shader_defs.push("DEPTH_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::MOTION_VECTOR_PREPASS) {
            shader_defs.push("MOTION_VECTOR_PREPASS".into());
        }

        if key.contains(MeshPipelineKey::SCREEN_SPACE_REFLECTIONS) {
            shader_defs.push("SCREEN_SPACE_REFLECTIONS".into());
        }

        if key.contains(MeshPipelineKey::CONTACT_SHADOWS) {
            shader_defs.push("CONTACT_SHADOWS".into());
        }

        if key.contains(MeshPipelineKey::HAS_PREVIOUS_SKIN) {
            shader_defs.push("HAS_PREVIOUS_SKIN".into());
        }

        if key.contains(MeshPipelineKey::HAS_PREVIOUS_MORPH) {
            shader_defs.push("HAS_PREVIOUS_MORPH".into());
        }

        if key.contains(MeshPipelineKey::DISTANCE_FOG) {
            shader_defs.push("DISTANCE_FOG".into());
        }
        if key.contains(MeshPipelineKey::ATMOSPHERE) {
            shader_defs.push("ATMOSPHERE".into());
        }
        shader_defs.push("STANDARD_MATERIAL_CLEARCOAT".into());

        // Always true, since we're in the deferred lighting pipeline
        shader_defs.push("DEFERRED_PREPASS".into());

        let shadow_filter_method =
            key.intersection(MeshPipelineKey::SHADOW_FILTER_METHOD_RESERVED_BITS);
        if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_HARDWARE_2X2 {
            shader_defs.push("SHADOW_FILTER_METHOD_HARDWARE_2X2".into());
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_GAUSSIAN {
            shader_defs.push("SHADOW_FILTER_METHOD_GAUSSIAN".into());
        } else if shadow_filter_method == MeshPipelineKey::SHADOW_FILTER_METHOD_TEMPORAL {
            shader_defs.push("SHADOW_FILTER_METHOD_TEMPORAL".into());
        }
        if self.mesh_pipeline.binding_arrays_are_usable {
            shader_defs.push("MULTIPLE_LIGHT_PROBES_IN_ARRAY".into());
            shader_defs.push("MULTIPLE_LIGHTMAPS_IN_ARRAY".into());
        }

        if IRRADIANCE_VOLUMES_ARE_USABLE {
            shader_defs.push("IRRADIANCE_VOLUMES_ARE_USABLE".into());
        }

        if self.mesh_pipeline.clustered_decals_are_usable {
            shader_defs.push("CLUSTERED_DECALS_ARE_USABLE".into());
            if cfg!(feature = "pbr_light_textures") {
                shader_defs.push("LIGHT_TEXTURES".into());
            }
        }

        #[cfg(feature = "experimental_pbr_pcss")]
        shader_defs.push("PCSS_SAMPLERS_AVAILABLE".into());

        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        shader_defs.push("SIXTEEN_BYTE_ALIGNMENT".into());

        if cfg!(feature = "bluenoise_texture") {
            shader_defs.push("BLUE_NOISE_TEXTURE".into());
        }
        if cfg!(feature = "dfg_lut") {
            shader_defs.push("DFG_LUT".into());
        }
        if cfg!(feature = "area_light_luts") {
            shader_defs.push("AREA_LIGHT_LUTS".into());
        }

        let layout = self.mesh_pipeline.get_view_layout(key.into());
        RenderPipelineDescriptor {
            label: Some("deferred_lighting_pipeline".into()),
            layout: vec![
                layout.main_layout,
                layout.binding_array_layout,
                self.bind_group_layout_2.clone(),
            ],
            vertex: VertexState {
                shader: self.deferred_lighting_shader.clone(),
                shader_defs: shader_defs.clone(),
                ..default()
            },
            fragment: Some(FragmentState {
                shader: self.deferred_lighting_shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: key.target_format(),
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            depth_stencil: Some(DepthStencilState {
                format: DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT,
                depth_write_enabled: Some(false),
                depth_compare: Some(CompareFunction::Equal),
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
            ..default()
        }
    }
}

pub fn init_deferred_lighting_layout(
    mut commands: Commands,
    mesh_pipeline: Res<MeshPipeline>,
    asset_server: Res<AssetServer>,
) {
    let layout = BindGroupLayoutDescriptor::new(
        "deferred_lighting_layout",
        &BindGroupLayoutEntries::single(
            ShaderStages::VERTEX_FRAGMENT,
            uniform_buffer::<PbrDeferredLightingDepthId>(false),
        ),
    );
    commands.insert_resource(DeferredLightingLayout {
        mesh_pipeline: mesh_pipeline.clone(),
        bind_group_layout_2: layout,
        deferred_lighting_shader: load_embedded_asset!(
            asset_server.as_ref(),
            "deferred_lighting.wesl"
        ),
    });
}

pub fn insert_deferred_lighting_pass_id_component(
    mut commands: Commands,
    views: Query<Entity, (With<DeferredPrepass>, Without<PbrDeferredLightingDepthId>)>,
) {
    for entity in views.iter() {
        commands
            .entity(entity)
            .insert(PbrDeferredLightingDepthId::default());
    }
}

pub fn prepare_deferred_lighting_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    view_key_cache: Res<ViewKeyCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<DeferredLightingLayout>>,
    deferred_lighting_layout: Res<DeferredLightingLayout>,
    cameras: Query<
        (
            Entity,
            &ExtractedView,
            Has<DeferredPrepass>,
            Has<SkipDeferredLighting>,
        ),
        With<ExtractedCamera>,
    >,
) {
    for (entity, view, deferred_prepass, skip_deferred_lighting) in &cameras {
        // If there is no deferred prepass or we want to skip the deferred lighting pass,
        // remove the old pipeline if there was one. This handles the case in which a
        // view using deferred stops using it.
        if !deferred_prepass || skip_deferred_lighting {
            commands.entity(entity).remove::<DeferredLightingPipeline>();
            continue;
        }

        // The deferred lighting pass runs the same lighting code as the forward
        // pass, so it must be specialized with the same view key. Reusing
        // [`ViewKeyCache`] keeps the two in sync automatically.
        let Some(view_key) = view_key_cache.get(&view.retained_view_entity) else {
            continue;
        };

        let pipeline_id =
            pipelines.specialize(&pipeline_cache, &deferred_lighting_layout, *view_key);

        commands
            .entity(entity)
            .insert(DeferredLightingPipeline { pipeline_id });
    }
}

/// Component to skip running the deferred lighting pass in [`deferred_lighting`] for a specific view.
///
/// This works like [`crate::PbrPlugin::add_default_deferred_lighting_plugin`], but is per-view instead of global.
///
/// Useful for cases where you want to generate a gbuffer, but skip the built-in deferred lighting pass
/// to run your own custom lighting pass instead.
///
/// Insert this component in the render world only.
#[derive(Component, Clone, Copy, Default)]
pub struct SkipDeferredLighting;
