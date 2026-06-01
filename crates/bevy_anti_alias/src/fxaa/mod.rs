use bevy_app::prelude::*;
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
use bevy_camera::Camera;
use bevy_core_pipeline::{
    schedule::{Core2d, Core2dSystems, Core3d, Core3dSystems},
    tonemapping::tonemapping,
    FullscreenShader,
};
use bevy_ecs::prelude::*;
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::ExtractedCamera,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_resource::{
        binding_types::{sampler, texture_2d, texture_2d_array},
        *,
    },
    renderer::RenderDevice,
    view::{ExtractedMultiview, ExtractedView},
    GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;
use core::num::NonZeroU32;

mod node;

pub use node::fxaa;

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
            .init_gpu_resource::<SpecializedRenderPipelines<FxaaPipeline>>()
            .add_systems(RenderStartup, init_fxaa_pipeline)
            .add_systems(
                Render,
                prepare_fxaa_pipelines.in_set(RenderSystems::Prepare),
            )
            .add_systems(
                Core3d,
                fxaa.after(tonemapping).in_set(Core3dSystems::PostProcess),
            )
            .add_systems(
                Core2d,
                fxaa.after(tonemapping).in_set(Core2dSystems::PostProcess),
            );
    }
}

#[derive(Resource)]
pub struct FxaaPipeline {
    pub texture_bind_group: BindGroupLayoutDescriptor,
    /// Multiview bind-group layout — the texture binding is a
    /// `texture_2d_array` whose layer is picked from `@builtin(view_index)`.
    pub texture_bind_group_multiview: BindGroupLayoutDescriptor,
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

    let texture_bind_group_multiview = BindGroupLayoutDescriptor::new(
        "fxaa_texture_bind_group_layout_multiview",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d_array(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    let sampler = render_device.create_sampler(&SamplerDescriptor {
        mipmap_filter: MipmapFilterMode::Linear,
        mag_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        ..default()
    });

    commands.insert_resource(FxaaPipeline {
        texture_bind_group,
        texture_bind_group_multiview,
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
    target_format: TextureFormat,
    /// Source texture layer count. `> 1` picks the `texture_2d_array`
    /// layout and emits `MULTIVIEW` + `MAX_VIEW_COUNT` shader-defs.
    multiview_view_count: u32,
}

impl SpecializedRenderPipeline for FxaaPipeline {
    type Key = FxaaPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![
            format!("EDGE_THRESH_{}", key.edge_threshold.get_str()).into(),
            format!("EDGE_THRESH_MIN_{}", key.edge_threshold_min.get_str()).into(),
        ];

        let layout = if key.multiview_view_count > 1 {
            shader_defs.push("MULTIVIEW".into());
            shader_defs.push(ShaderDefVal::UInt(
                "MAX_VIEW_COUNT".into(),
                key.multiview_view_count,
            ));
            self.texture_bind_group_multiview.clone()
        } else {
            self.texture_bind_group.clone()
        };

        // Broadcast across every eye layer in a single pass. The matching
        // render-pass descriptor in `node.rs` sets the same mask. The mask
        // is `(1 << view_count) - 1` (one bit per eye); computed via
        // `u32::MAX >> (32 - view_count)` to avoid the shift overflow that
        // `1 << 32` would hit at the `MAX_VIEW_COUNT` cap.
        let multiview_mask = if key.multiview_view_count > 1 {
            NonZeroU32::new(u32::MAX >> (32 - key.multiview_view_count))
        } else {
            None
        };

        RenderPipelineDescriptor {
            label: Some("fxaa".into()),
            multiview_mask,
            layout: vec![layout],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: key.target_format,
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
    cameras: Query<
        (Entity, &ExtractedView, &Fxaa, Option<&ExtractedMultiview>),
        With<ExtractedCamera>,
    >,
) {
    for (entity, view, fxaa, multiview) in &cameras {
        if !fxaa.enabled {
            continue;
        }
        let multiview_view_count = multiview.map_or(1, |m| m.subviews.len() as u32);
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &fxaa_pipeline,
            FxaaPipelineKey {
                edge_threshold: fxaa.edge_threshold,
                edge_threshold_min: fxaa.edge_threshold_min,
                target_format: view.target_format,
                multiview_view_count,
            },
        );

        commands
            .entity(entity)
            .insert(CameraFxaaPipeline { pipeline_id });
    }
}
