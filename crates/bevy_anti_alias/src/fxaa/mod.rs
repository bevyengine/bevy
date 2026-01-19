use crate::AntiAliasing;
use bevy_app::prelude::*;
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::Camera;
use bevy_core_pipeline::{
    schedule::{Core2d, Core2dSystems, Core3d, Core3dSystems},
    tonemapping::tonemapping,
    FullscreenShader,
};
use bevy_ecs::prelude::*;
use bevy_image::BevyDefault as _;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_resource::{
        binding_types::{sampler, texture_2d},
        *,
    },
    renderer::RenderDevice,
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::Shader;
use bevy_utils::default;

mod node;

pub(crate) use node::fxaa;

#[derive(Debug, Reflect, Eq, PartialEq, Hash, Clone, Copy)]
#[reflect(PartialEq, Hash, Clone)]
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

/// A component for enabling Fast Approximate Anti-Aliasing (FXAA)
/// for a [`bevy_camera::Camera`].
#[derive(Reflect, Component, Clone, ExtractComponent)]
#[reflect(Component, Default, Clone)]
#[extract_component_filter(With<Camera>)]
#[doc(alias = "FastApproximateAntiAliasing")]
pub struct Fxaa {
    /// Enable render passes for FXAA.
    pub enabled: bool,

    /// Use lower sensitivity for a sharper, faster, result.
    /// Use higher sensitivity for a slower, smoother, result.
    /// [`Ultra`](`Sensitivity::Ultra`) and [`Extreme`](`Sensitivity::Extreme`)
    /// settings can result in significant smearing and loss of detail.
    ///
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

/// Adds support for Fast Approximate Anti-Aliasing (FXAA)
#[derive(Default)]
pub struct FxaaPlugin;
impl Plugin for FxaaPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "fxaa.wgsl");

        app.add_plugins(ExtractComponentPlugin::<Fxaa>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<FxaaPipeline>>()
            .add_systems(RenderStartup, init_fxaa_pipeline)
            .add_systems(
                Render,
                prepare_fxaa_pipelines.in_set(RenderSystems::Prepare),
            )
            .add_systems(
                Core3d,
                fxaa.after(tonemapping)
                    .before(Core3dSystems::EndMainPassPostProcessing)
                    .in_set(AntiAliasing),
            )
            .add_systems(
                Core2d,
                fxaa.after(tonemapping)
                    .before(Core2dSystems::EndMainPassPostProcessing)
                    .in_set(AntiAliasing),
            );
    }
}

#[derive(Resource)]
pub struct FxaaPipeline {
    texture_bind_group: BindGroupLayoutDescriptor,
    sampler: Sampler,
    fullscreen_shader: FullscreenShader,
    fragment_shader: Handle<Shader>,
}

pub fn init_fxaa_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let texture_bind_group = BindGroupLayoutDescriptor::new(
        "fxaa_texture_bind_group_layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
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

    commands.insert_resource(FxaaPipeline {
        texture_bind_group,
        sampler,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "fxaa.wgsl"),
    });
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
            layout: vec![self.texture_bind_group.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs: vec![
                    format!("EDGE_THRESH_{}", key.edge_threshold.get_str()).into(),
                    format!("EDGE_THRESH_MIN_{}", key.edge_threshold_min.get_str()).into(),
                ],
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

pub fn prepare_fxaa_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<FxaaPipeline>>,
    fxaa_pipeline: Res<FxaaPipeline>,
    views: Query<(Entity, &ExtractedView, &Fxaa)>,
) {
    for (entity, view, fxaa) in &views {
        if !fxaa.enabled {
            continue;
        }
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
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
