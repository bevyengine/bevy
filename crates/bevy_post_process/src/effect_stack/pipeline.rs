use bevy_asset::{load_embedded_asset, AssetServer, Handle};
use bevy_core_pipeline::FullscreenShader;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{Or, With},
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
};
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        BindGroupLayoutDescriptor, BindGroupLayoutEntries, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, FilterMode, FragmentState, MipmapFilterMode, PipelineCache,
        RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, ShaderStages,
        SpecializedRenderPipeline, SpecializedRenderPipelines, TextureFormat, TextureSampleType,
    },
    renderer::RenderDevice,
    view::ExtractedView,
};
use bevy_shader::Shader;
use bevy_utils::default;

use crate::effect_stack::{
    ChromaticAberration, ChromaticAberrationUniform, LensDistortion, LensDistortionUniform,
    Vignette, VignetteUniform,
};

/// GPU pipeline data for the built-in postprocessing stack.
///
/// This is stored in the render world.
#[derive(Resource)]
pub struct PostProcessingPipeline {
    /// The layout of bind group 0, containing the source, LUT, and settings.
    pub bind_group_layout: BindGroupLayoutDescriptor,
    /// A shared sampler used to sample both the source framebuffer texture and the LUT texture.
    pub common_sampler: Sampler,
    /// The asset handle for the fullscreen vertex shader.
    pub fullscreen_shader: FullscreenShader,
    /// The fragment shader asset handle.
    pub fragment_shader: Handle<Shader>,
}

/// A key that uniquely identifies a built-in postprocessing pipeline.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostProcessingPipelineKey {
    /// The format of the source and destination textures.
    pub target_format: TextureFormat,
}

/// A component attached to cameras in the render world that stores the
/// specialized pipeline ID for the built-in postprocessing stack.
#[derive(Component, Deref, DerefMut)]
pub struct PostProcessingPipelineId(pub CachedRenderPipelineId);

pub fn init_post_processing_pipeline(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    fullscreen_shader: Res<FullscreenShader>,
    asset_server: Res<AssetServer>,
) {
    // Create our single bind group layout.
    let bind_group_layout = BindGroupLayoutDescriptor::new(
        "postprocessing bind group layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                // Common source:
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Common sampler:
                sampler(SamplerBindingType::Filtering),
                // Chromatic aberration LUT:
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Chromatic aberration settings:
                uniform_buffer::<ChromaticAberrationUniform>(true),
                // Vignette settings:
                uniform_buffer::<VignetteUniform>(true),
                // Lens Distortion settings:
                uniform_buffer::<LensDistortionUniform>(true),
            ),
        ),
    );

    // Both source and chromatic aberration LUTs should be sampled
    // bilinearly.
    let common_sampler = render_device.create_sampler(&SamplerDescriptor {
        mipmap_filter: MipmapFilterMode::Linear,
        min_filter: FilterMode::Linear,
        mag_filter: FilterMode::Linear,
        ..default()
    });

    commands.insert_resource(PostProcessingPipeline {
        bind_group_layout,
        common_sampler,
        fullscreen_shader: fullscreen_shader.clone(),
        fragment_shader: load_embedded_asset!(asset_server.as_ref(), "post_process.wgsl"),
    });
}

impl SpecializedRenderPipeline for PostProcessingPipeline {
    type Key = PostProcessingPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("postprocessing".into()),
            layout: vec![self.bind_group_layout.clone()],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
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

/// Specializes the built-in postprocessing pipeline for each applicable view.
pub(crate) fn prepare_post_processing_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PostProcessingPipeline>>,
    post_processing_pipeline: Res<PostProcessingPipeline>,
    cameras: Query<
        (Entity, &ExtractedView),
        Or<(
            With<ChromaticAberration>,
            With<Vignette>,
            With<LensDistortion>,
            With<ExtractedCamera>,
        )>,
    >,
) {
    for (entity, view) in cameras.iter() {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &post_processing_pipeline,
            PostProcessingPipelineKey {
                target_format: view.target_format,
            },
        );

        commands
            .entity(entity)
            .insert(PostProcessingPipelineId(pipeline_id));
    }
}
