use bevy_app::prelude::*;
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::Camera;
use bevy_core_pipeline::{
    core_2d::graph::{Core2d, Node2d},
    core_3d::graph::{Core3d, Node3d},
    FullscreenShader,
};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_image::BevyDefault as _;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
    render_graph::RenderGraphExt,
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        *,
    },
    renderer::RenderDevice,
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::Shader;
use bevy_utils::default;

mod node;

pub use node::CasNode;

/// Applies a contrast adaptive sharpening (CAS) filter to the camera.
///
/// CAS is usually used in combination with shader based anti-aliasing methods
/// such as FXAA or TAA to regain some of the lost detail from the blurring that they introduce.
///
/// CAS is designed to adjust the amount of sharpening applied to different areas of an image
/// based on the local contrast. This can help avoid over-sharpening areas with high contrast
/// and under-sharpening areas with low contrast.
///
/// To use this, add the [`ContrastAdaptiveSharpening`] component to a 2D or 3D camera.
#[derive(Component, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
pub struct ContrastAdaptiveSharpening {
    /// Enable or disable sharpening.
    pub enabled: bool,
    /// Adjusts sharpening strength. Higher values increase the amount of sharpening.
    ///
    /// Clamped between 0.0 and 1.0.
    ///
    /// The default value is 0.6.
    pub sharpening_strength: f32,
    /// Whether to try and avoid sharpening areas that are already noisy.
    ///
    /// You probably shouldn't use this, and just leave it set to false.
    /// You should generally apply any sort of film grain or similar effects after CAS
    /// and upscaling to avoid artifacts.
    pub denoise: bool,
}

impl Default for ContrastAdaptiveSharpening {
    fn default() -> Self {
        ContrastAdaptiveSharpening {
            enabled: true,
            sharpening_strength: 0.6,
            denoise: false,
        }
    }
}

#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component, Default, Clone)]
pub struct DenoiseCas(bool);

/// The uniform struct extracted from [`ContrastAdaptiveSharpening`] attached to a [`Camera`].
/// Will be available for use in the CAS shader.
#[doc(hidden)]
#[derive(Component, ShaderType, Clone)]
pub struct CasUniform {
    sharpness: f32,
}

impl ExtractComponent for ContrastAdaptiveSharpening {
    type QueryData = &'static Self;
    type QueryFilter = With<Camera>;
    type Out = (DenoiseCas, CasUniform);

    fn extract_component(item: QueryItem<Self::QueryData>) -> Option<Self::Out> {
        if !item.enabled || item.sharpening_strength == 0.0 {
            return None;
        }
        Some((
            DenoiseCas(item.denoise),
            CasUniform {
                // above 1.0 causes extreme artifacts and fireflies
                sharpness: item.sharpening_strength.clamp(0.0, 1.0),
            },
        ))
    }
}

/// Adds Support for Contrast Adaptive Sharpening (CAS).
#[derive(Default)]
pub struct CasPlugin;

impl Plugin for CasPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "robust_contrast_adaptive_sharpening.wgsl");

        app.add_plugins((
            ExtractComponentPlugin::<ContrastAdaptiveSharpening>::default(),
            UniformComponentPlugin::<CasUniform>::default(),
        ));

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<CasPipeline>>()
            .add_systems(RenderStartup, init_cas_pipeline)
            .add_systems(Render, prepare_cas_pipelines.in_set(RenderSystems::Prepare));

        {
            render_app
                .add_render_graph_node::<CasNode>(Core3d, Node3d::ContrastAdaptiveSharpening)
                .add_render_graph_edge(
                    Core3d,
                    Node3d::Tonemapping,
                    Node3d::ContrastAdaptiveSharpening,
                )
                .add_render_graph_edges(
                    Core3d,
                    (
                        Node3d::Fxaa,
                        Node3d::ContrastAdaptiveSharpening,
                        Node3d::EndMainPassPostProcessing,
                    ),
                );
        }
        {
            render_app
                .add_render_graph_node::<CasNode>(Core2d, Node2d::ContrastAdaptiveSharpening)
                .add_render_graph_edge(
                    Core2d,
                    Node2d::Tonemapping,
                    Node2d::ContrastAdaptiveSharpening,
                )
                .add_render_graph_edges(
                    Core2d,
                    (
                        Node2d::Fxaa,
                        Node2d::ContrastAdaptiveSharpening,
                        Node2d::EndMainPassPostProcessing,
                    ),
                );
        }
    }
}

#[derive(Resource)]
pub struct CasPipeline {
    texture_bind_group: BindGroupLayout,
    sampler: Sampler,
    fullscreen_shader: FullscreenShader,
    fragment_shader: Handle<Shader>,
}

pub fn init_cas_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let texture_bind_group = render_device.create_bind_group_layout(
        "sharpening_texture_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
                // CAS Settings
                uniform_buffer::<CasUniform>(true),
            ),
        ),
    );

    let sampler = render_device.create_sampler(&SamplerDescriptor::default());

    commands.insert_resource(CasPipeline {
        texture_bind_group,
        sampler,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(
            asset_server.as_ref(),
            "robust_contrast_adaptive_sharpening.wgsl"
        ),
    });
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct CasPipelineKey {
    texture_format: TextureFormat,
    denoise: bool,
}

impl SpecializedRenderPipeline for CasPipeline {
    type Key = CasPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![];
        if key.denoise {
            shader_defs.push("RCAS_DENOISE".into());
        }
        RenderPipelineDescriptor {
            label: Some("contrast_adaptive_sharpening".into()),
            layout: vec![self.texture_bind_group.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

fn prepare_cas_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<CasPipeline>>,
    sharpening_pipeline: Res<CasPipeline>,
    views: Query<
        (Entity, &ExtractedView, &DenoiseCas),
        Or<(Added<CasUniform>, Changed<DenoiseCas>)>,
    >,
    mut removals: RemovedComponents<CasUniform>,
) {
    for entity in removals.read() {
        commands.entity(entity).remove::<ViewCasPipeline>();
    }

    for (entity, view, denoise_cas) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &sharpening_pipeline,
            CasPipelineKey {
                denoise: denoise_cas.0,
                texture_format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
            },
        );

        commands.entity(entity).insert(ViewCasPipeline(pipeline_id));
    }
}

#[derive(Component)]
pub struct ViewCasPipeline(CachedRenderPipelineId);
