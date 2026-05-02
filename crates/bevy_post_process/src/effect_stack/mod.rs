//! Miscellaneous built-in postprocessing effects.
//!
//! Includes:
//!
//! - Chromatic Aberration
//! - Lens Distortion
//! - Vignette

mod chromatic_aberration;
mod lens_distortion;
mod pipeline;
mod vignette;

pub use chromatic_aberration::{ChromaticAberration, ChromaticAberrationUniform};
pub use lens_distortion::{LensDistortion, LensDistortionUniform};
pub use vignette::{Vignette, VignetteUniform};

use crate::effect_stack::{
    chromatic_aberration::{DefaultChromaticAberrationLut, DEFAULT_CHROMATIC_ABERRATION_LUT_DATA},
    pipeline::{
        init_post_processing_pipeline, prepare_post_processing_pipelines, PostProcessingPipeline,
        PostProcessingPipelineId,
    },
};

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, Assets, RenderAssetUsages};
use bevy_color::ColorToComponents;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{AnyOf, Or, With},
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Commands, Query, Res, ResMut},
};
use bevy_image::Image;
use bevy_math::Vec2;
use bevy_render::{
    diagnostic::RecordDiagnostics,
    extract_component::ExtractComponentPlugin,
    render_asset::RenderAssets,
    render_resource::{
        BindGroupEntries, DynamicUniformBuffer, Extent3d, Operations, PipelineCache,
        RenderPassColorAttachment, RenderPassDescriptor, SpecializedRenderPipelines,
        TextureDimension, TextureFormat,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue, ViewQuery},
    texture::GpuImage,
    view::{ExtractedView, ViewTarget},
    GpuResourceAppExt, Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::load_shader_library;

use crate::{bloom::bloom, dof::depth_of_field};
use bevy_core_pipeline::{
    schedule::{Core2d, Core3d},
    tonemapping::tonemapping,
};

/// A plugin that implements a built-in postprocessing stack with some common
/// effects.
///
/// Includes:
///
/// - Chromatic Aberration
/// - Lens Distortion
/// - Vignette
#[derive(Default)]
pub struct EffectStackPlugin;

/// A resource, part of the render world, that stores the uniform buffers for
/// post-processing effects.
///
/// This currently holds buffers, allowing them to be uploaded to the GPU efficiently.
#[derive(Resource, Default)]
pub struct PostProcessingUniformBuffers {
    chromatic_aberration: DynamicUniformBuffer<ChromaticAberrationUniform>,
    vignette: DynamicUniformBuffer<VignetteUniform>,
    lens_distortion: DynamicUniformBuffer<LensDistortionUniform>,
}

/// A component, part of the render world, that stores the appropriate byte
/// offset within the [`PostProcessingUniformBuffers`] for the camera it's
/// attached to.
#[derive(Component)]
pub struct PostProcessingUniformBufferOffsets {
    chromatic_aberration: u32,
    vignette: u32,
    lens_distortion: u32,
}

impl Plugin for EffectStackPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "chromatic_aberration.wgsl");
        load_shader_library!(app, "lens_distortion.wgsl");
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
            .add_plugins(ExtractComponentPlugin::<LensDistortion>::default())
            .add_plugins(ExtractComponentPlugin::<Vignette>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .insert_resource(DefaultChromaticAberrationLut(default_lut))
            .init_gpu_resource::<SpecializedRenderPipelines<PostProcessingPipeline>>()
            .init_gpu_resource::<PostProcessingUniformBuffers>()
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

fn post_processing(
    view: ViewQuery<(
        &ViewTarget,
        &PostProcessingPipelineId,
        AnyOf<(&ChromaticAberration, &Vignette, &LensDistortion)>,
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

    let (maybe_chromatic_aberration, maybe_vignette, maybe_lens_distortion) = post_effects;

    if maybe_chromatic_aberration.is_none()
        && maybe_vignette.is_none()
        && maybe_lens_distortion.is_none()
    {
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

    let Some(lens_distortion_uniform_buffer_binding) =
        post_processing_uniform_buffers.lens_distortion.binding()
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
            &post_processing_pipeline.common_sampler,
            &chromatic_aberration_lut.texture_view,
            chromatic_aberration_uniform_buffer_binding,
            vignette_uniform_buffer_binding,
            lens_distortion_uniform_buffer_binding,
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
            post_processing_uniform_buffer_offsets.lens_distortion,
        ],
    );
    render_pass.draw(0..3, 0..1);

    pass_span.end(&mut render_pass);
}

/// Gathers the built-in postprocessing settings for every view and uploads them
/// to the GPU.
fn prepare_post_processing_uniforms(
    mut commands: Commands,
    mut post_processing_uniform_buffers: ResMut<PostProcessingUniformBuffers>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut views: Query<
        (
            Entity,
            &ExtractedView,
            Option<&ChromaticAberration>,
            Option<&Vignette>,
            Option<&LensDistortion>,
        ),
        Or<(
            With<ChromaticAberration>,
            With<Vignette>,
            With<LensDistortion>,
        )>,
    >,
) {
    post_processing_uniform_buffers.chromatic_aberration.clear();
    post_processing_uniform_buffers.vignette.clear();
    post_processing_uniform_buffers.lens_distortion.clear();

    // Gather up all the postprocessing settings.
    for (view_entity, view, maybe_chromatic_aberration, maybe_vignette, maybe_lens_distortion) in
        views.iter_mut()
    {
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

        let vignette_uniform_buffer_offset = if let (Some(vignette), view_size) =
            (maybe_vignette, view.viewport)
        {
            let width = view_size.z as f32;
            let height = view_size.w as f32;

            let screen_aspect = width / height;
            let aspect_ratio_vec = Vec2::new(1.0, height / width);
            let uv_offset = (vignette.center - Vec2::new(0.5, 0.5)) * aspect_ratio_vec;

            let min_dim = width.min(height);
            let norm_aspect_ratio = Vec2::new(width / min_dim, height / min_dim);
            let base_scale = norm_aspect_ratio
                * Vec2::new(1.0, 1.0 / vignette.roundness.clamp(1e-6, 2.0 - 1e-6));
            // e1 * (1.0 - e3) + e2 * e3, where e1 = 1.0
            let edge_factor = if screen_aspect >= 1.0 {
                Vec2::new(
                    1.0 - vignette.edge_compensation
                        + screen_aspect.recip() * vignette.edge_compensation,
                    1.0,
                )
            } else {
                Vec2::new(
                    1.0,
                    1.0 - vignette.edge_compensation + screen_aspect * vignette.edge_compensation,
                )
            };
            let uv_scale = base_scale * edge_factor;

            post_processing_uniform_buffers
                .vignette
                .push(&VignetteUniform {
                    intensity: vignette.intensity.min(1.0),
                    inv_radius: 1.0 / vignette.radius.max(1e-6),
                    smoothness: vignette.smoothness.max(0.0),
                    unused: 0,
                    uv_offset,
                    uv_scale,
                    color: vignette.color.to_srgba().to_vec4(),
                })
        } else {
            post_processing_uniform_buffers
                .vignette
                .push(&VignetteUniform::default())
        };

        let lens_distortion_uniform_buffer_offset =
            if let Some(lens_distortion) = maybe_lens_distortion {
                post_processing_uniform_buffers
                    .lens_distortion
                    .push(&LensDistortionUniform {
                        intensity: lens_distortion.intensity,
                        inv_scale: 1.0 / lens_distortion.scale.max(1e-6),
                        multiplier: lens_distortion.multiplier,
                        center: lens_distortion.center,
                        edge_intensity: lens_distortion.intensity * lens_distortion.edge_curvature,
                        unused: 0,
                    })
            } else {
                post_processing_uniform_buffers
                    .lens_distortion
                    .push(&LensDistortionUniform::default())
            };

        commands
            .entity(view_entity)
            .insert(PostProcessingUniformBufferOffsets {
                chromatic_aberration: chromatic_aberration_uniform_buffer_offset,
                vignette: vignette_uniform_buffer_offset,
                lens_distortion: lens_distortion_uniform_buffer_offset,
            });
    }

    // Upload to the GPU.
    post_processing_uniform_buffers
        .chromatic_aberration
        .write_buffer(&render_device, &render_queue);
    post_processing_uniform_buffers
        .vignette
        .write_buffer(&render_device, &render_queue);
    post_processing_uniform_buffers
        .lens_distortion
        .write_buffer(&render_device, &render_queue);
}
