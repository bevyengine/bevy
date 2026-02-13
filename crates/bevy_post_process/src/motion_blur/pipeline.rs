use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_core_pipeline::FullscreenShader;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::BevyDefault as _;
use bevy_render::{
    globals::GlobalsUniform,
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer_sized},
        BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, FilterMode, FragmentState, PipelineCache,
        RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
        ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat,
        TextureSampleType,
    },
    renderer::RenderDevice,
    settings::WgpuFeatures,
    view::{ExtractedView, ViewTarget},
};
use bevy_shader::Shader;
use bevy_utils::default;

use super::MotionBlurUniform;

#[derive(Resource)]
pub struct MotionBlurPipeline {
    pub(crate) sampler: Sampler,
    pub(crate) layout: BindGroupLayoutDescriptor,
    pub(crate) fullscreen_shader: FullscreenShader,
    pub(crate) fragment_shader: Handle<Shader>,
}

impl MotionBlurPipeline {
    pub(crate) fn new(
        render_device: &RenderDevice,
        fullscreen_shader: FullscreenShader,
        fragment_shader: Handle<Shader>,
    ) -> Self {
        let depth_filterable = render_device
            .features()
            .contains(WgpuFeatures::FLOAT32_FILTERABLE);
        let mb_layout = &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // View target (read)
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Motion Vectors
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Depth
                texture_2d(TextureSampleType::Float {
                    filterable: depth_filterable,
                }),
                // Linear Sampler
                sampler(SamplerBindingType::Filtering),
                // Motion blur settings uniform input
                uniform_buffer_sized(false, Some(MotionBlurUniform::min_size())),
                // Globals uniform input
                uniform_buffer_sized(false, Some(GlobalsUniform::min_size())),
            ),
        );
        let filter_mode = if depth_filterable {
            FilterMode::Linear
        } else {
            FilterMode::Nearest
        };
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            min_filter: filter_mode,
            mag_filter: filter_mode,
            ..Default::default()
        });
        let layout = BindGroupLayoutDescriptor::new("motion_blur_layout", mb_layout);

        Self {
            sampler,
            layout,
            fullscreen_shader,
            fragment_shader,
        }
    }
}

pub fn init_motion_blur_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    let fullscreen_shader = fullscreen_shader.clone();
    let fragment_shader = load_embedded_asset!(asset_server.as_ref(), "motion_blur.wgsl");
    commands.insert_resource(MotionBlurPipeline::new(
        &render_device,
        fullscreen_shader,
        fragment_shader,
    ));
}

#[derive(PartialEq, Eq, Hash, Clone, Copy)]
pub struct MotionBlurPipelineKey {
    hdr: bool,
}

impl SpecializedRenderPipeline for MotionBlurPipeline {
    type Key = MotionBlurPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let layout = vec![self.layout.clone()];
        #[cfg(not(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu"))))]
        let shader_defs = vec![];
        #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
        let shader_defs = vec!["SIXTEEN_BYTE_ALIGNMENT".into()];

        RenderPipelineDescriptor {
            label: Some("motion_blur_pipeline".into()),
            layout,
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: if key.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            ..default()
        }
    }
}

#[derive(Component)]
pub struct MotionBlurPipelineId(pub CachedRenderPipelineId);

pub(crate) fn prepare_motion_blur_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<MotionBlurPipeline>>,
    pipeline: Res<MotionBlurPipeline>,
    views: Query<(Entity, &ExtractedView), With<MotionBlurUniform>>,
) {
    for (entity, view) in &views {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &pipeline,
            MotionBlurPipelineKey { hdr: view.hdr },
        );

        commands
            .entity(entity)
            .insert(MotionBlurPipelineId(pipeline_id));
    }
}
