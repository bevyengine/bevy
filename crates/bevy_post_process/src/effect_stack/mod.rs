//! Miscellaneous built-in postprocessing effects.
//!
//! Currently, this consists only of chromatic aberration.

use bevy_app::{App, Plugin};
use bevy_asset::{
    embedded_asset, load_embedded_asset, AssetServer, Assets, Handle, RenderAssetUsages,
};
use bevy_camera::Camera;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryItem, With},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut},
};
use bevy_image::{BevyDefault, Image};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    diagnostic::RecordDiagnostics,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::RenderAssets,
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, DynamicUniformBuffer, Extent3d,
        FilterMode, FragmentState, Operations, PipelineCache, RenderPassColorAttachment,
        RenderPassDescriptor, RenderPipelineDescriptor, Sampler, SamplerBindingType,
        SamplerDescriptor, ShaderStages, ShaderType, SpecializedRenderPipeline,
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

/// The default chromatic aberration intensity amount, in a fraction of the
/// window size.
const DEFAULT_CHROMATIC_ABERRATION_INTENSITY: f32 = 0.02;

/// The default maximum number of samples for chromatic aberration.
const DEFAULT_CHROMATIC_ABERRATION_MAX_SAMPLES: u32 = 8;

/// The raw RGBA data for the default chromatic aberration gradient.
///
/// This consists of one red pixel, one green pixel, and one blue pixel, in that
/// order.
static DEFAULT_CHROMATIC_ABERRATION_LUT_DATA: [u8; 12] =
    [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];

#[derive(Resource)]
pub(crate) struct DefaultChromaticAberrationLut(Handle<Image>);

/// A plugin that implements a built-in postprocessing stack with some common
/// effects.
///
/// Currently, this only consists of chromatic aberration.
#[derive(Default)]
pub struct EffectStackPlugin;

/// Adds colored fringes to the edges of objects in the scene.
///
/// [Chromatic aberration] simulates the effect when lenses fail to focus all
/// colors of light toward a single point. It causes rainbow-colored streaks to
/// appear, which are especially apparent on the edges of objects. Chromatic
/// aberration is commonly used for collision effects, especially in horror
/// games.
///
/// Bevy's implementation is based on that of *Inside* ([Gjøl & Svendsen 2016]).
/// It's based on a customizable lookup texture, which allows for changing the
/// color pattern. By default, the color pattern is simply a 3×1 pixel texture
/// consisting of red, green, and blue, in that order, but you can change it to
/// any image in order to achieve different effects.
///
/// [Chromatic aberration]: https://en.wikipedia.org/wiki/Chromatic_aberration
///
/// [Gjøl & Svendsen 2016]: https://github.com/playdeadgames/publications/blob/master/INSIDE/rendering_inside_gdc2016.pdf
#[derive(Reflect, Component, Clone)]
#[reflect(Component, Default, Clone)]
pub struct ChromaticAberration {
    /// The lookup texture that determines the color gradient.
    ///
    /// By default (if None), this is a 3×1 texel texture consisting of one red
    /// pixel, one green pixel, and one blue texel, in that order. This
    /// recreates the most typical chromatic aberration pattern. However, you
    /// can change it to achieve different artistic effects.
    ///
    /// The texture is always sampled in its vertical center, so it should
    /// ordinarily have a height of 1 texel.
    pub color_lut: Option<Handle<Image>>,

    /// The size of the streaks around the edges of objects, as a fraction of
    /// the window size.
    ///
    /// The default value is 0.02.
    pub intensity: f32,

    /// A cap on the number of texture samples that will be performed.
    ///
    /// Higher values result in smoother-looking streaks but are slower.
    ///
    /// The default value is 8.
    pub max_samples: u32,
}

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

/// The on-GPU version of the [`ChromaticAberration`] settings.
///
/// See the documentation for [`ChromaticAberration`] for more information on
/// each of these fields.
#[derive(ShaderType)]
pub struct ChromaticAberrationUniform {
    /// The intensity of the effect, in a fraction of the screen.
    intensity: f32,
    /// A cap on the number of samples of the source texture that the shader
    /// will perform.
    max_samples: u32,
    /// Padding data.
    unused_1: u32,
    /// Padding data.
    unused_2: u32,
}

/// A resource, part of the render world, that stores the
/// [`ChromaticAberrationUniform`]s for each view.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct PostProcessingUniformBuffers {
    chromatic_aberration: DynamicUniformBuffer<ChromaticAberrationUniform>,
}

/// A component, part of the render world, that stores the appropriate byte
/// offset within the [`PostProcessingUniformBuffers`] for the camera it's
/// attached to.
#[derive(Component, Deref, DerefMut)]
pub struct PostProcessingUniformBufferOffsets {
    chromatic_aberration: u32,
}

impl Plugin for EffectStackPlugin {
    fn build(&self, app: &mut App) {
        load_shader_library!(app, "chromatic_aberration.wgsl");

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

        app.add_plugins(ExtractComponentPlugin::<ChromaticAberration>::default());

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
            .add_systems(Core3d, post_processing.after(depth_of_field).before(tonemapping))
            .add_systems(Core2d, post_processing.after(bloom).before(tonemapping));
    }
}

impl Default for ChromaticAberration {
    fn default() -> Self {
        Self {
            color_lut: None,
            intensity: DEFAULT_CHROMATIC_ABERRATION_INTENSITY,
            max_samples: DEFAULT_CHROMATIC_ABERRATION_MAX_SAMPLES,
        }
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
                // Chromatic aberration source:
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Chromatic aberration source sampler:
                sampler(SamplerBindingType::Filtering),
                // Chromatic aberration LUT:
                texture_2d(TextureSampleType::Float { filterable: true }),
                // Chromatic aberration LUT sampler:
                sampler(SamplerBindingType::Filtering),
                // Chromatic aberration settings:
                uniform_buffer::<ChromaticAberrationUniform>(true),
            ),
        ),
    );

    // Both source and chromatic aberration LUTs should be sampled
    // bilinearly.

    let source_sampler = render_device.create_sampler(&SamplerDescriptor {
        mipmap_filter: FilterMode::Linear,
        min_filter: FilterMode::Linear,
        mag_filter: FilterMode::Linear,
        ..default()
    });

    let chromatic_aberration_lut_sampler = render_device.create_sampler(&SamplerDescriptor {
        mipmap_filter: FilterMode::Linear,
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
        &ChromaticAberration,
        &PostProcessingUniformBufferOffsets,
    )>,
    pipeline_cache: Res<PipelineCache>,
    post_processing_pipeline: Res<PostProcessingPipeline>,
    post_processing_uniform_buffers: Res<PostProcessingUniformBuffers>,
    gpu_image_assets: Res<RenderAssets<GpuImage>>,
    default_lut: Res<DefaultChromaticAberrationLut>,
    mut ctx: RenderContext,
) {
    let (view_target, pipeline_id, chromatic_aberration, post_processing_uniform_buffer_offsets) =
        view.into_inner();

    // We need a render pipeline to be prepared.
    let Some(pipeline) = pipeline_cache.get_render_pipeline(**pipeline_id) else {
        return;
    };

    // We need the chromatic aberration LUT to be present.
    let Some(chromatic_aberration_lut) = gpu_image_assets.get(
        chromatic_aberration
            .color_lut
            .as_ref()
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
        )),
    );

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();
    let time_span = diagnostics.time_span(ctx.command_encoder(), "postprocessing");

    {
        let mut render_pass = ctx.command_encoder().begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[**post_processing_uniform_buffer_offsets]);
        render_pass.draw(0..3, 0..1);
    }

    time_span.end(ctx.command_encoder());
}

/// Specializes the built-in postprocessing pipeline for each applicable view.
pub fn prepare_post_processing_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PostProcessingPipeline>>,
    post_processing_pipeline: Res<PostProcessingPipeline>,
    views: Query<(Entity, &ExtractedView), With<ChromaticAberration>>,
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
    mut views: Query<(Entity, &ChromaticAberration)>,
) {
    post_processing_uniform_buffers.clear();

    // Gather up all the postprocessing settings.
    for (view_entity, chromatic_aberration) in views.iter_mut() {
        let chromatic_aberration_uniform_buffer_offset =
            post_processing_uniform_buffers.push(&ChromaticAberrationUniform {
                intensity: chromatic_aberration.intensity,
                max_samples: chromatic_aberration.max_samples,
                unused_1: 0,
                unused_2: 0,
            });
        commands
            .entity(view_entity)
            .insert(PostProcessingUniformBufferOffsets {
                chromatic_aberration: chromatic_aberration_uniform_buffer_offset,
            });
    }

    // Upload to the GPU.
    post_processing_uniform_buffers.write_buffer(&render_device, &render_queue);
}

impl ExtractComponent for ChromaticAberration {
    type QueryData = Read<ChromaticAberration>;

    type QueryFilter = With<Camera>;

    type Out = ChromaticAberration;

    fn extract_component(
        chromatic_aberration: QueryItem<'_, '_, Self::QueryData>,
    ) -> Option<Self::Out> {
        // Skip the postprocessing phase entirely if the intensity is zero.
        if chromatic_aberration.intensity > 0.0 {
            Some(chromatic_aberration.clone())
        } else {
            None
        }
    }
}
