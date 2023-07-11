use crate::{
    core_2d::{self, CORE_2D},
    core_3d::{self, CORE_3D},
    fullscreen_vertex_shader::fullscreen_shader_vertex_state,
};
use bevy_app::prelude::*;
use bevy_asset::{load_internal_asset, Handle};
use bevy_ecs::{prelude::*, query::QueryItem};
use bevy_reflect::Reflect;
use bevy_render::{
    extract_component::{ExtractComponent, ExtractComponentPlugin, UniformComponentPlugin},
    prelude::Camera,
    render_graph::RenderGraphApp,
    render_resource::*,
    renderer::RenderDevice,
    texture::BevyDefault,
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderSet,
};

mod node;

pub use node::CASNode;

/// Applies a contrast adaptive sharpening (CAS) filter to the camera.
///
/// CAS is usually used in combination with shader based anti-aliasing methods
/// such as FXAA or TAA to regain some of the lost detail from the blurring that they introduce.
///
/// CAS is designed to adjust the amount of sharpening applied to different areas of an image
/// based on the local contrast. This can help avoid over-sharpening areas with high contrast
/// and under-sharpening areas with low contrast.
///
/// To use this, add the [`ContrastAdaptiveSharpeningSettings`] component to a 2D or 3D camera.
#[derive(Component, Reflect, Clone)]
#[reflect(Component)]
pub struct ContrastAdaptiveSharpeningSettings {
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

impl Default for ContrastAdaptiveSharpeningSettings {
    fn default() -> Self {
        ContrastAdaptiveSharpeningSettings {
            enabled: true,
            sharpening_strength: 0.6,
            denoise: false,
        }
    }
}

#[derive(Component, Default, Reflect, Clone)]
#[reflect(Component)]
pub struct DenoiseCAS(bool);

/// The uniform struct extracted from [`ContrastAdaptiveSharpeningSettings`] attached to a [`Camera`].
/// Will be available for use in the CAS shader.
#[doc(hidden)]
#[derive(Component, ShaderType, Clone)]
pub struct CASUniform {
    sharpness: f32,
}

impl ExtractComponent for ContrastAdaptiveSharpeningSettings {
    type Query = &'static Self;
    type Filter = With<Camera>;
    type Out = (DenoiseCAS, CASUniform);

    fn extract_component(item: QueryItem<Self::Query>) -> Option<Self::Out> {
        if !item.enabled || item.sharpening_strength == 0.0 {
            return None;
        }
        Some((
            DenoiseCAS(item.denoise),
            CASUniform {
                // above 1.0 causes extreme artifacts and fireflies
                sharpness: item.sharpening_strength.clamp(0.0, 1.0),
            },
        ))
    }
}

const CONTRAST_ADAPTIVE_SHARPENING_SHADER_HANDLE: Handle<Shader> =
    Handle::weak_from_u128(6925381244141981602);

/// Adds Support for Contrast Adaptive Sharpening (CAS).
pub struct CASPlugin;

impl Plugin for CASPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            CONTRAST_ADAPTIVE_SHARPENING_SHADER_HANDLE,
            "robust_contrast_adaptive_sharpening.wgsl",
            Shader::from_wgsl
        );

        app.register_type::<ContrastAdaptiveSharpeningSettings>();
        app.add_plugins((
            ExtractComponentPlugin::<ContrastAdaptiveSharpeningSettings>::default(),
            UniformComponentPlugin::<CASUniform>::default(),
        ));

        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };
        render_app
            .init_resource::<SpecializedRenderPipelines<CASPipeline>>()
            .add_systems(Render, prepare_cas_pipelines.in_set(RenderSet::Prepare));

        {
            use core_3d::graph::node::*;
            render_app
                .add_render_graph_node::<CASNode>(CORE_3D, CONTRAST_ADAPTIVE_SHARPENING)
                .add_render_graph_edge(CORE_3D, TONEMAPPING, CONTRAST_ADAPTIVE_SHARPENING)
                .add_render_graph_edges(
                    CORE_3D,
                    &[
                        FXAA,
                        CONTRAST_ADAPTIVE_SHARPENING,
                        END_MAIN_PASS_POST_PROCESSING,
                    ],
                );
        }
        {
            use core_2d::graph::node::*;
            render_app
                .add_render_graph_node::<CASNode>(CORE_2D, CONTRAST_ADAPTIVE_SHARPENING)
                .add_render_graph_edge(CORE_2D, TONEMAPPING, CONTRAST_ADAPTIVE_SHARPENING)
                .add_render_graph_edges(
                    CORE_2D,
                    &[
                        FXAA,
                        CONTRAST_ADAPTIVE_SHARPENING,
                        END_MAIN_PASS_POST_PROCESSING,
                    ],
                );
        }
    }

    fn finish(&self, app: &mut App) {
        let render_app = match app.get_sub_app_mut(RenderApp) {
            Ok(render_app) => render_app,
            Err(_) => return,
        };
        render_app.init_resource::<CASPipeline>();
    }
}

#[derive(Resource)]
pub struct CASPipeline {
    texture_bind_group: BindGroupLayout,
    sampler: Sampler,
}

impl FromWorld for CASPipeline {
    fn from_world(render_world: &mut World) -> Self {
        let render_device = render_world.resource::<RenderDevice>();
        let texture_bind_group =
            render_device.create_bind_group_layout(&BindGroupLayoutDescriptor {
                label: Some("sharpening_texture_bind_group_layout"),
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
                    // CAS Settings
                    BindGroupLayoutEntry {
                        binding: 2,
                        ty: BindingType::Buffer {
                            ty: BufferBindingType::Uniform,
                            has_dynamic_offset: true,
                            min_binding_size: Some(CASUniform::min_size()),
                        },
                        visibility: ShaderStages::FRAGMENT,
                        count: None,
                    },
                ],
            });

        let sampler = render_device.create_sampler(&SamplerDescriptor::default());

        CASPipeline {
            texture_bind_group,
            sampler,
        }
    }
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct CASPipelineKey {
    texture_format: TextureFormat,
    denoise: bool,
}

impl SpecializedRenderPipeline for CASPipeline {
    type Key = CASPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = vec![];
        if key.denoise {
            shader_defs.push("RCAS_DENOISE".into());
        }
        RenderPipelineDescriptor {
            label: Some("contrast_adaptive_sharpening".into()),
            layout: vec![self.texture_bind_group.clone()],
            vertex: fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: CONTRAST_ADAPTIVE_SHARPENING_SHADER_HANDLE,
                shader_defs,
                entry_point: "fragment".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: PrimitiveState::default(),
            depth_stencil: None,
            multisample: MultisampleState::default(),
            push_constant_ranges: Vec::new(),
        }
    }
}

fn prepare_cas_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<CASPipeline>>,
    sharpening_pipeline: Res<CASPipeline>,
    views: Query<(Entity, &ExtractedView, &DenoiseCAS), With<CASUniform>>,
) {
    for (entity, view, cas_settings) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &sharpening_pipeline,
            CASPipelineKey {
                denoise: cas_settings.0,
                texture_format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
            },
        );

        commands.entity(entity).insert(ViewCASPipeline(pipeline_id));
    }
}

#[derive(Component)]
pub struct ViewCASPipeline(CachedRenderPipelineId);
