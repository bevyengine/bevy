use crate::{MeshPipeline, MeshViewBindGroup, ScreenSpaceAmbientOcclusionSettings};
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, Handle};
use bevy_core_pipeline::{
    clear_color::ClearColorConfig,
    core_3d,
    deferred::{
        copy_lighting_id::DeferredLightingIdDepthTexture, DEFERRED_LIGHTING_PASS_ID_DEPTH_FORMAT,
    },
    prelude::{Camera3d, ClearColor},
    prepass::{DeferredPrepass, DepthPrepass, MotionVectorPrepass, NormalPrepass},
    tonemapping::{DebandDither, Tonemapping},
};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_render::{
    extract_component::{
        ComponentUniforms, ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin,
    },
    render_asset::RenderAssets,
    render_graph::{NodeRunError, RenderGraphContext, ViewNode, ViewNodeRunner},
    render_resource::{self, Operations, PipelineCache, RenderPassDescriptor},
    renderer::{RenderContext, RenderDevice},
    texture::Image,
    view::{ViewTarget, ViewUniformOffset},
    Render, RenderSet,
};

use bevy_render::{
    render_graph::RenderGraphApp, render_resource::*, texture::BevyDefault, view::ExtractedView,
    RenderApp,
};

use crate::{
    EnvironmentMapLight, MeshPipelineKey, ShadowFilteringMethod, ViewFogUniformOffset,
    ViewLightsUniformOffset,
};

pub struct DeferredPbrLightingPlugin;

pub const DEFERRED_LIGHTING_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(2708011359337029741);

pub const DEFAULT_PBR_DEFERRED_LIGHTING_PASS_ID: u8 = 1;

/// Component with a `depth_id` for specifying which corresponding materials should be rendered by this specific PBR deferred lighting pass.
/// Will be automatically added to entities with the [`DeferredPrepass`] component that don't already have a [`PbrDeferredLightingDepthId`].
#[derive(Component, Clone, Copy, ExtractComponent, ShaderType)]
pub struct PbrDeferredLightingDepthId {
    depth_id: u32,

    #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
    _webgl2_padding_0: f32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
    _webgl2_padding_1: f32,
    #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
    _webgl2_padding_2: f32,
}

impl PbrDeferredLightingDepthId {
    pub fn new(value: u8) -> PbrDeferredLightingDepthId {
        PbrDeferredLightingDepthId {
            depth_id: value as u32,

            #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
            _webgl2_padding_0: 0.0,
            #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
            _webgl2_padding_1: 0.0,
            #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
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

            #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
            _webgl2_padding_0: 0.0,
            #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
            _webgl2_padding_1: 0.0,
            #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
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
            .add_render_graph_node::<ViewNodeRunner<DeferredOpaquePass3dPbrLightingNode>>(
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

pub const DEFERRED_LIGHTING_PASS: &str = "deferred_opaque_pbr_lighting_pass_3d";
#[derive(Default)]
pub struct DeferredOpaquePass3dPbrLightingNode;

impl ViewNode for DeferredOpaquePass3dPbrLightingNode {
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
        let deferred_lighting_layout = world.resource::<DeferredLightingLayout>();

        let Some(pipeline) =
            pipeline_cache.get_render_pipeline(deferred_lighting_pipeline.pipeline_id)
        else {
            return Ok(());
        };

        let deferred_lighting_pass_id =
            world.resource::<ComponentUniforms<PbrDeferredLightingDepthId>>();
        let Some(deferred_lighting_pass_id_binding) =
            deferred_lighting_pass_id.uniforms().binding()
        else {
            return Ok(());
        };

        let bind_group_1 = render_context.render_device().create_bind_group(
            "deferred_lighting_layout_group_1",
            &deferred_lighting_layout.bind_group_layout_1,
            &BindGroupEntries::single(deferred_lighting_pass_id_binding),
        );

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
        render_pass.set_bind_group(1, &bind_group_1, &[]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

#[derive(Resource)]
pub struct DeferredLightingLayout {
    mesh_pipeline: MeshPipeline,
    bind_group_layout_1: BindGroupLayout,
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

        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
        shader_defs.push("WEBGL2".into());

        if key.tonemap_in_shader() {
            shader_defs.push("TONEMAP_IN_SHADER".into());
            shader_defs.push(key.tonemap_method().define().into());

            // Debanding is tied to tonemapping in the shader, cannot run without it.
            if key.deband_dither() {
                shader_defs.push("DEBAND_DITHER".into());
            }
        }

        if key.screen_space_ambient_occlusion() {
            shader_defs.push("SCREEN_SPACE_AMBIENT_OCCLUSION".into());
        }

        if key.environment_map() {
            shader_defs.push("ENVIRONMENT_MAP".into());
        }

        if key.normal_prepass() {
            shader_defs.push("NORMAL_PREPASS".into());
        }

        if key.depth_prepass() {
            shader_defs.push("DEPTH_PREPASS".into());
        }

        if key.motion_vector_prepass() {
            shader_defs.push("MOTION_VECTOR_PREPASS".into());
        }

        // Always true, since we're in the deferred lighting pipeline
        shader_defs.push("DEFERRED_PREPASS".into());

        if let Ok(filter_method) = key.shadow_filter_method() {
            shader_defs.push(filter_method.define().into());
        }

        #[cfg(all(feature = "webgl", target_arch = "wasm32"))]
        shader_defs.push("SIXTEEN_BYTE_ALIGNMENT".into());

        RenderPipelineDescriptor {
            label: Some("deferred_lighting_pipeline".into()),
            layout: vec![
                self.mesh_pipeline.get_view_layout(key.into()).clone(),
                self.bind_group_layout_1.clone(),
            ],
            vertex: VertexState {
                shader: DEFERRED_LIGHTING_SHADER_HANDLE,
                shader_defs: shader_defs.clone(),
                entry_point: "vertex".into(),
                buffers: Vec::new(),
            },
            fragment: Some(FragmentState {
                shader: DEFERRED_LIGHTING_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr() {
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
        let render_device = world.resource::<RenderDevice>();
        let layout = render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("deferred_lighting_layout"),
            entries: &[BindGroupLayoutEntry {
                binding: 0,
                visibility: ShaderStages::VERTEX_FRAGMENT,
                ty: BindingType::Buffer {
                    ty: render_resource::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: Some(PbrDeferredLightingDepthId::min_size()),
                },
                count: None,
            }],
        });
        Self {
            mesh_pipeline: world.resource::<MeshPipeline>().clone(),
            bind_group_layout_1: layout,
        }
    }
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
    mut pipelines: ResMut<SpecializedRenderPipelines<DeferredLightingLayout>>,
    deferred_lighting_layout: Res<DeferredLightingLayout>,
    views: Query<
        (
            Entity,
            &ExtractedView,
            Option<&Tonemapping>,
            Option<&DebandDither>,
            Option<&EnvironmentMapLight>,
            Option<&ShadowFilteringMethod>,
            Option<&ScreenSpaceAmbientOcclusionSettings>,
            (
                Has<NormalPrepass>,
                Has<DepthPrepass>,
                Has<MotionVectorPrepass>,
            ),
        ),
        With<DeferredPrepass>,
    >,
    images: Res<RenderAssets<Image>>,
) {
    for (
        entity,
        view,
        tonemapping,
        dither,
        environment_map,
        shadow_filter_method,
        ssao,
        (normal_prepass, depth_prepass, motion_vector_prepass),
    ) in &views
    {
        let environment_map_loaded = environment_map.is_some_and(|e| e.is_loaded(&images));
        let filtering_method = shadow_filter_method.copied().unwrap_or_default();

        let mut view_key = MeshPipelineKey::DEFAULT
            .with_hdr(view.hdr)
            .with_normal_prepass(normal_prepass)
            .with_depth_prepass(depth_prepass)
            .with_motion_vector_prepass(motion_vector_prepass)
            .with_screen_space_ambient_occlusion(ssao.is_some())
            .with_environment_map(environment_map_loaded)
            .with_shadow_filter_method(filtering_method)
            // Always true, since we're in the deferred lighting pipeline
            .with_deferred_prepass(true);

        if !view.hdr {
            if let Some(tonemapping) = tonemapping {
                view_key = view_key
                    .with_tonemap_in_shader(true)
                    .with_tonemap_method(*tonemapping);
            }
            let deband_dither = dither.is_some_and(|m| matches!(m, DebandDither::Enabled));
            view_key = view_key.with_deband_dither(deband_dither);
        }

        let pipeline_id =
            pipelines.specialize(&pipeline_cache, &deferred_lighting_layout, view_key);

        commands
            .entity(entity)
            .insert(DeferredLightingPipeline { pipeline_id });
    }
}
