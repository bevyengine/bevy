//! Miscellaneous built-in postprocessing effects.
//!
//! Includes:
//!
//! - Chromatic Aberration
//! - Vignette

mod chromatic_aberration;
mod vignette;

use bevy_color::ColorToComponents;
pub use chromatic_aberration::{ChromaticAberration, ChromaticAberrationUniform};
pub use vignette::{Vignette, VignetteUniform};

use crate::effect_stack::chromatic_aberration::{
    DefaultChromaticAberrationLut, DEFAULT_CHROMATIC_ABERRATION_LUT_DATA,
};

use bevy_app::{App, Plugin};
use bevy_asset::{
    embedded_asset, load_embedded_asset, AssetServer, Assets, Handle, RenderAssetUsages,
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{AnyOf, Or, With},
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::{BevyDefault, Image};
use bevy_render::{
    diagnostic::RecordDiagnostics,
    extract_component::ExtractComponentPlugin,
    render_asset::RenderAssets,
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, DynamicUniformBuffer, Extent3d,
        FilterMode, FragmentState, MipmapFilterMode, Operations, PipelineCache,
        RenderPassColorAttachment, RenderPassDescriptor, RenderPipelineDescriptor, Sampler,
        SamplerBindingType, SamplerDescriptor, ShaderStages, SpecializedRenderPipeline,
        SpecializedRenderPipelines, TextureDimension, TextureFormat, TextureSampleType,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
    texture::GpuImage,
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::{load_shader_library, Shader};
use bevy_utils::prelude::default;

use crate::{bloom::bloom, dof::depth_of_field};
use bevy_core_pipeline::{
    schedule::{Core2d, Core3d},
    tonemapping::tonemapping,
    FullscreenShader,
};

/// A plugin that implements a built-in postprocessing stack with some common
/// effects.
///
/// Includes:
///
/// - Chromatic Aberration
/// - Vignette
#[derive(Default)]
pub struct EffectStackPlugin;

/// GPU pipeline data for the built-in postprocessing stack.
///
/// This is stored in the render world.
#[derive(Resource)]
pub struct PostProcessingPipeline {
    /// The layout of bind group 0, containing the source, LUT, and settings.
    bind_group_layout: BindGroupLayoutDescriptor,
    /// Specifies how to sample the source framebuffer texture.
    source_sampler: Sampler,
    /// Specifies how to sample the chromatic aberration gradient.
    chromatic_aberration_lut_sampler: Sampler,
    /// The asset handle for the fullscreen vertex shader.
    fullscreen_shader: FullscreenShader,
    /// The fragment shader asset handle.
    fragment_shader: Handle<Shader>,
}

/// A key that uniquely identifies a built-in postprocessing pipeline.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostProcessingPipelineKey {
    /// The format of the source and destination textures.
    texture_format: TextureFormat,
}

/// A component attached to cameras in the render world that stores the
/// specialized pipeline ID for the built-in postprocessing stack.
#[derive(Component, Deref, DerefMut)]
pub struct PostProcessingPipelineId(CachedRenderPipelineId);

/// A resource, part of the render world, that stores the uniform buffers for
/// post-processing effects.
///
/// This currently holds buffers for [`ChromaticAberrationUniform`] and
/// [`VignetteUniform`], allowing them to be uploaded to the GPU efficiently.
#[derive(Resource, Default)]
pub struct PostProcessingUniformBuffers {
    chromatic_aberration: DynamicUniformBuffer<ChromaticAberrationUniform>,
    vignette: DynamicUniformBuffer<VignetteUniform>,
}

/// A component, part of the render world, that stores the appropriate byte
/// offset within the [`PostProcessingUniformBuffers`] for the camera it's
/// attached to.
#[derive(Component)]
pub struct PostProcessingUniformBufferOffsets {
    chromatic_aberration: u32,
    vignette: u32,
}

impl Plugin for EffectStackPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "chromatic_aberration.wgsl");
        load_shader_library!(app, "vignette.wgsl");

        embedded_asset!(app, "post_process.wgsl");

        // Load the default chromatic aberration LUT.
        let mut assets = app.world_mut().resource_mut::<Assets<_>>();
        let default_lut = assets.add(Image::new(
            Extent3d {
                width: 3,
                height: 1,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            DEFAULT_CHROMATIC_ABERRATION_LUT_DATA.to_vec(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::RENDER_WORLD,
        ));

        app.add_plugins(ExtractComponentPlugin::<ChromaticAberration>::default())
            .add_plugins(ExtractComponentPlugin::<Vignette>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .insert_resource(DefaultChromaticAberrationLut(default_lut))
            .init_resource::<SpecializedRenderPipelines<PostProcessingPipeline>>()
            .init_resource::<PostProcessingUniformBuffers>()
            .add_systems(RenderStartup, init_post_processing_pipeline)
            .add_systems(
                Render,
                (
                    prepare_post_processing_pipelines,
                    prepare_post_processing_uniforms,
                )
                    .in_set(RenderSystems::Prepare),
            )
            .add_systems(
                Core3d,
                post_processing.after(depth_of_field).before(tonemapping),
            )
            .add_systems(Core2d, post_processing.after(bloom).before(tonemapping));
    }
}

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
                // Common source sampler:
                sampler(SamplerBindingType::Filtering),
                // Chromatic aberration LUT:
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Chromatic aberration LUT sampler:
                sampler(SamplerBindingType::Filtering),
                // Chromatic aberration settings:
                uniform_buffer::<ChromaticAberrationUniform>(true),
                // Vignette settings:
                uniform_buffer::<VignetteUniform>(true),
            ),
        ),
    );

    // Both source and chromatic aberration LUTs should be sampled
    // bilinearly.

    let source_sampler = render_device.create_sampler(&SamplerDescriptor {
        mipmap_filter: MipmapFilterMode::Linear,
        min_filter: FilterMode::Linear,
        mag_filter: FilterMode::Linear,
        ..default()
    });

    let chromatic_aberration_lut_sampler = render_device.create_sampler(&SamplerDescriptor {
        mipmap_filter: MipmapFilterMode::Linear,
        min_filter: FilterMode::Linear,
        mag_filter: FilterMode::Linear,
        ..default()
    });

    commands.insert_resource(PostProcessingPipeline {
        bind_group_layout,
        source_sampler,
        chromatic_aberration_lut_sampler,
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

pub(crate) fn post_processing(
    view: ViewQuery<(
        &ViewTarget,
        &PostProcessingPipelineId,
        AnyOf<(&ChromaticAberration, &Vignette)>,
        &PostProcessingUniformBufferOffsets,
    )>,
    pipeline_cache: Res<PipelineCache>,
    post_processing_pipeline: Res<PostProcessingPipeline>,
    post_processing_uniform_buffers: Res<PostProcessingUniformBuffers>,
    gpu_image_assets: Res<RenderAssets<GpuImage>>,
    default_lut: Res<DefaultChromaticAberrationLut>,
    mut ctx: RenderContext,
) {
    let (view_target, pipeline_id, post_effects, post_processing_uniform_buffer_offsets) =
        view.into_inner();

    let (maybe_chromatic_aberration, maybe_vignette) = post_effects;

    if maybe_chromatic_aberration.is_none() && maybe_vignette.is_none() {
        return;
    }

    // We need a render pipeline to be prepared.
    let Some(pipeline) = pipeline_cache.get_render_pipeline(**pipeline_id) else {
        return;
    };

    // We need the chromatic aberration LUT to be present.
    let Some(chromatic_aberration_lut) = gpu_image_assets.get(
        maybe_chromatic_aberration
            .and_then(|ca| ca.color_lut.as_ref())
            .unwrap_or(&default_lut.0),
    ) else {
        return;
    };

    // We need the postprocessing settings to be uploaded to the GPU.
    let Some(chromatic_aberration_uniform_buffer_binding) = post_processing_uniform_buffers
        .chromatic_aberration
        .binding()
    else {
        return;
    };

    let Some(vignette_uniform_buffer_binding) = post_processing_uniform_buffers.vignette.binding()
    else {
        return;
    };

    // Use the [`PostProcessWrite`] infrastructure, since this is a full-screen pass.
    let post_process = view_target.post_process_write();

    let pass_descriptor = RenderPassDescriptor {
        label: Some("postprocessing"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: post_process.destination,
            depth_slice: None,
            resolve_target: None,
            ops: Operations::default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    };

    let bind_group = ctx.render_device().create_bind_group(
        Some("postprocessing bind group"),
        &pipeline_cache.get_bind_group_layout(&post_processing_pipeline.bind_group_layout),
        &BindGroupEntries::sequential((
            post_process.source,
            &post_processing_pipeline.source_sampler,
            &chromatic_aberration_lut.texture_view,
            &post_processing_pipeline.chromatic_aberration_lut_sampler,
            chromatic_aberration_uniform_buffer_binding,
            vignette_uniform_buffer_binding,
        )),
    );

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    let mut render_pass = ctx.begin_tracked_render_pass(pass_descriptor);
    let pass_span = diagnostics.pass_span(&mut render_pass, "postprocessing");

    render_pass.set_render_pipeline(pipeline);
    render_pass.set_bind_group(
        0,
        &bind_group,
        &[
            post_processing_uniform_buffer_offsets.chromatic_aberration,
            post_processing_uniform_buffer_offsets.vignette,
        ],
    );
    render_pass.draw(0..3, 0..1);

    pass_span.end(&mut render_pass);
}

/// Specializes the built-in postprocessing pipeline for each applicable view.
pub fn prepare_post_processing_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PostProcessingPipeline>>,
    post_processing_pipeline: Res<PostProcessingPipeline>,
    views: Query<(Entity, &ExtractedView), Or<(With<ChromaticAberration>, With<Vignette>)>>,
) {
    for (entity, view) in views.iter() {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &post_processing_pipeline,
            PostProcessingPipelineKey {
                texture_format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
            },
        );

        commands
            .entity(entity)
            .insert(PostProcessingPipelineId(pipeline_id));
    }
}

/// Gathers the built-in postprocessing settings for every view and uploads them
/// to the GPU.
pub fn prepare_post_processing_uniforms(
    mut commands: Commands,
    mut post_processing_uniform_buffers: ResMut<PostProcessingUniformBuffers>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut views: Query<
        (Entity, Option<&ChromaticAberration>, Option<&Vignette>),
        Or<(With<ChromaticAberration>, With<Vignette>)>,
    >,
) {
    post_processing_uniform_buffers.chromatic_aberration.clear();
    post_processing_uniform_buffers.vignette.clear();

    // Gather up all the postprocessing settings.
    for (view_entity, maybe_chromatic_aberration, maybe_vignette) in views.iter_mut() {
        let chromatic_aberration_uniform_buffer_offset =
            if let Some(chromatic_aberration) = maybe_chromatic_aberration {
                post_processing_uniform_buffers.chromatic_aberration.push(
                    &ChromaticAberrationUniform {
                        intensity: chromatic_aberration.intensity,
                        max_samples: chromatic_aberration.max_samples,
                        unused_1: 0,
                        unused_2: 0,
                    },
                )
            } else {
                post_processing_uniform_buffers
                    .chromatic_aberration
                    .push(&ChromaticAberrationUniform::default())
            };

        let vignette_uniform_buffer_offset = if let Some(vignette) = maybe_vignette {
            post_processing_uniform_buffers
                .vignette
                .push(&VignetteUniform {
                    intensity: vignette.intensity,
                    radius: vignette.radius,
                    smoothness: vignette.smoothness,
                    roundness: vignette.roundness,
                    center: vignette.center,
                    edge_compensation: vignette.edge_compensation,
                    unused: 0,
                    color: vignette.color.to_srgba().to_vec4(),
                })
        } else {
            post_processing_uniform_buffers
                .vignette
                .push(&VignetteUniform::default())
        };

        commands
            .entity(view_entity)
            .insert(PostProcessingUniformBufferOffsets {
                chromatic_aberration: chromatic_aberration_uniform_buffer_offset,
                vignette: vignette_uniform_buffer_offset,
            });
    }

    // Upload to the GPU.
    post_processing_uniform_buffers
        .chromatic_aberration
        .write_buffer(&render_device, &render_queue);
    post_processing_uniform_buffers
        .vignette
        .write_buffer(&render_device, &render_queue);
}
