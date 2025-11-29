use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_ecs::prelude::*;
use bevy_image::{BevyDefault, ToExtents};
use bevy_reflect::Reflect;
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_graph::{RenderGraphExt, RenderLabel, ViewNodeRunner},
    render_resource::{
        binding_types::{sampler, texture_2d},
        BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
        BlendComponent, BlendFactor, BlendOperation, BlendState, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, FilterMode, FragmentState, PipelineCache,
        RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines, TextureDescriptor, TextureDimension,
        TextureFormat, TextureSampleType, TextureUsages,
    },
    renderer::RenderDevice,
    texture::{CachedTexture, TextureCache},
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::Shader;
use bevy_utils::default;

use crate::{
    core_3d::{
        graph::{Core3d, Node3d},
        MainWbOitPass3dNode,
    },
    oit::node::{WbOitClearPassNode, WbOitResolveNode},
    FullscreenShader,
};

pub mod node;

#[derive(Component, ExtractComponent, Clone, Copy, Default, Reflect)]
#[reflect(Component, Clone)]
pub struct WeightedBlendedOit;

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct WbOitMainPass;

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct WbOitResolvePass;

#[derive(RenderLabel, Debug, Clone, Hash, PartialEq, Eq)]
pub struct WbOitClearPass;

/// Weighted Blended Order Independent Transparency (WBOIT)
///
/// To enable this for a camera you need to add the [`WeightedBlendedOit`] component to it and set material alpha mode to `AlphaMode::WeightedBlend`.
///
/// Compared to [`ExactOitPlugin`](crate::oit::ExactOitPlugin), this uses constant memory that is independent of layers and does not require sorting in the shader, potentially offering better performance. One limitation of WBOIT is that it can produce inaccurate blending in scenes with higher depth/alpha complexity, particularly when alpha is close to opacity.
///
/// This can't be used with [`ExactOitPlugin`](crate::oit::ExactOitPlugin), and can't be used with MSAA yet.
///
/// # Implementation details
/// See <https://learnopengl.com/Guest-Articles/2020/OIT/Weighted-Blended>. When used with custom materials, you need to write the weighted blended color and alpha to fragment output.
pub struct WeightedBlendedOitPlugin;
impl Plugin for WeightedBlendedOitPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "wboit_resolve.wgsl");

        app.add_plugins(ExtractComponentPlugin::<WeightedBlendedOit>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<WbOitResolvePipeline>>()
            .add_systems(RenderStartup, init_wboit_resolve_pipeline)
            .add_systems(
                Render,
                (
                    prepare_wboit_resolve_pipelines.in_set(RenderSystems::Prepare),
                    prepare_wboit_textures.in_set(RenderSystems::PrepareResources),
                    prepare_wboit_resolve_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<MainWbOitPass3dNode>>(Core3d, WbOitMainPass)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::MainTransparentPass,
                    WbOitMainPass,
                    Node3d::EndMainPass,
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<WbOitClearPassNode>>(Core3d, WbOitClearPass)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::StartMainPass,
                    WbOitClearPass,
                    Node3d::MainTransparentPass,
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<WbOitResolveNode>>(Core3d, WbOitResolvePass)
            .add_render_graph_edges(
                Core3d,
                (WbOitMainPass, WbOitResolvePass, Node3d::EndMainPass),
            );
    }
}

pub const WBOIT_ACCUM_TEXTURE_FORMAT: TextureFormat = TextureFormat::Rgba16Float;
pub const WBOIT_REVEAL_TEXTURE_FORMAT: TextureFormat = TextureFormat::R8Unorm;

#[derive(Component)]
pub struct WeightedBlendedOitTextures {
    pub accum: CachedTexture,
    pub reveal: CachedTexture,
}

#[derive(Component)]
pub struct WbOitResolvePipelineId(pub CachedRenderPipelineId);

#[derive(Resource)]
struct WbOitResolvePipeline {
    bind_group_layout: BindGroupLayoutDescriptor,
    sampler: Sampler,
    fullscreen_shader: FullscreenShader,
    fragment_shader: Handle<Shader>,
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
struct WbOitResolvePipelineKey {
    target_format: TextureFormat,
}

impl SpecializedRenderPipeline for WbOitResolvePipeline {
    type Key = WbOitResolvePipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("wboit_resolve".into()),
            layout: vec![self.bind_group_layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::OneMinusSrcAlpha,
                            dst_factor: BlendFactor::SrcAlpha,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::OneMinusSrcAlpha,
                            dst_factor: BlendFactor::SrcAlpha,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

#[derive(Component)]
pub struct WbOitResolveBindGroup(pub BindGroup);

fn prepare_wboit_textures(
    mut commands: Commands,
    mut texture_cache: ResMut<TextureCache>,
    render_device: Res<RenderDevice>,
    views: Query<(Entity, &ExtractedCamera), With<WeightedBlendedOit>>,
) {
    for (entity, camera) in &views {
        let Some(viewport) = camera.physical_viewport_size else {
            return;
        };

        let texture_descriptor = TextureDescriptor {
            label: Some("wboit_accum_texture"),
            size: viewport.to_extents(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: WBOIT_ACCUM_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let accum = texture_cache.get(&render_device, texture_descriptor);

        let texture_descriptor = TextureDescriptor {
            label: Some("wboit_reveal_texture"),
            size: viewport.to_extents(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: WBOIT_REVEAL_TEXTURE_FORMAT,
            usage: TextureUsages::RENDER_ATTACHMENT | TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        };
        let reveal = texture_cache.get(&render_device, texture_descriptor);

        commands
            .entity(entity)
            .insert(WeightedBlendedOitTextures { accum, reveal });
    }
}

fn init_wboit_resolve_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "wboit_resolve_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    let sampler = render_device.create_sampler(&SamplerDescriptor {
        mipmap_filter: FilterMode::Linear,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });

    commands.insert_resource(WbOitResolvePipeline {
        bind_group_layout,
        sampler,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "wboit_resolve.wgsl"),
    });
}

fn prepare_wboit_resolve_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<WbOitResolvePipeline>>,
    wboit_resolve_pipeline: Res<WbOitResolvePipeline>,
    views: Query<(Entity, &ExtractedView), With<WeightedBlendedOit>>,
) {
    for (entity, view) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &wboit_resolve_pipeline,
            WbOitResolvePipelineKey {
                target_format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
            },
        );

        commands
            .entity(entity)
            .insert(WbOitResolvePipelineId(pipeline_id));
    }
}

fn prepare_wboit_resolve_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    wboit_resolve_pipelines: Res<WbOitResolvePipeline>,
    pipeline_cache: Res<PipelineCache>,
    view_targets: Query<
        (Entity, &WeightedBlendedOitTextures),
        (With<ExtractedView>, With<WeightedBlendedOit>),
    >,
) {
    for (entity, wboit_textures) in &view_targets {
        commands
            .entity(entity)
            .insert(WbOitResolveBindGroup(render_device.create_bind_group(
                "wboit resolve bind group",
                &pipeline_cache.get_bind_group_layout(&wboit_resolve_pipelines.bind_group_layout),
                &BindGroupEntries::sequential((
                    &wboit_textures.accum.default_view,
                    &wboit_textures.reveal.default_view,
                    &wboit_resolve_pipelines.sampler,
                )),
            )));
    }
}
