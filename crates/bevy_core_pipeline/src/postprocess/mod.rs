//! Miscellaneous built-in postprocessing effects.
//!
//! Currently, this consists only of chromatic aberration.

use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, Assets, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryItem, With},
    reflect::ReflectComponent,
    schedule::IntoSystemConfigs as _,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut, Resource},
    world::{FromWorld, World},
};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::Camera,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::{RenderAssetUsages, RenderAssets},
    render_graph::{
        NodeRunError, RenderGraphApp as _, RenderGraphContext, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_1d, texture_2d, uniform_buffer},
        BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries, CachedRenderPipelineId,
        ColorTargetState, ColorWrites, DynamicUniformBuffer, Extent3d, FilterMode, FragmentState,
        Operations, PipelineCache, RenderPassColorAttachment, RenderPassDescriptor,
        RenderPipelineDescriptor, Sampler, SamplerBindingType, SamplerDescriptor, Shader,
        ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines,
        TextureDimension, TextureFormat, TextureSampleType,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::{BevyDefault, GpuImage, Image},
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderSet,
};
use bevy_utils::prelude::default;

use crate::{
    core_2d::graph::{Core2d, Node2d},
    core_3d::graph::{Core3d, Node3d},
    fullscreen_vertex_shader,
};

/// The handle to the built-in postprocessing shader `postprocess.wgsl`.
const POSTPROCESSING_SHADER_HANDLE: Handle<Shader> = Handle::weak_from_u128(14675654334038973533);

/// The handle to the default chromatic aberration lookup texture.
///
/// This is just a 3x1 image consisting of one red pixel, one green pixel, and
/// one blue pixel, in that order.
const DEFAULT_CHROMATIC_ABERRATION_LUT_HANDLE: Handle<Image> =
    Handle::weak_from_u128(2199972955136579180);

/// The default chromatic aberration intensity amount, in a fraction of the
/// window size.
const DEFAULT_CHROMATIC_ABBERATION_INTENSITY: f32 = 0.02;

/// The default maximum number of samples for chromatic aberration.
const DEFAULT_CHROMATIC_ABERRATION_MAX_SAMPLES: u32 = 8;

/// The raw RGBA data for the default chromatic aberration gradient.
///
/// This consists of one red pixel, one green pixel, and one blue pixel, in that
/// order.
static DEFAULT_CHROMATIC_ABBERATION_LUT_DATA: [u8; 12] =
    [255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255];

/// A plugin that implements a built-in postprocessing stack with some common
/// effects.
///
/// Currently, this only consists of chromatic aberration.
pub struct PostprocessingPlugin;

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
/// any 1D image in order to achieve different effects.
///
/// [Chromatic aberration]: https://en.wikipedia.org/wiki/Chromatic_aberration
///
/// [Gjøl & Svendsen 2016]: https://github.com/playdeadgames/publications/blob/master/INSIDE/rendering_inside_gdc2016.pdf
#[derive(Reflect, Component, Clone)]
#[reflect(Component, Default)]
pub struct ChromaticAberration {
    /// The 1D lookup texture that determines the color gradient.
    ///
    /// By default, this is a 3×1 pixel texture consisting of one red pixel, one
    /// green pixel, and one blue pixel, in that order. This recreates the most
    /// typical chromatic aberration pattern. However, you can change it to
    /// achieve different artistic effects.
    pub color_lut: Handle<Image>,

    /// The size of the streaks around the edges of objects, as a fraction of
    /// the window size.
    ///
    /// The default value is 0.2.
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
pub struct PostprocessingPipeline {
    /// The layout of bind group 0, containing the source, LUT, and settings.
    bind_group_layout: BindGroupLayout,
    /// Specifies how to sample the source framebuffer texture.
    source_sampler: Sampler,
    /// Specifies how to sample the chromatic aberration gradient.
    chromatic_aberration_lut_sampler: Sampler,
}

/// A key that uniquely identifies a built-in postprocessing pipeline.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostprocessingPipelineKey {
    /// The format of the source and destination textures.
    texture_format: TextureFormat,
}

/// A component attached to cameras in the render world that stores the
/// specialized pipeline ID for the built-in postprocessing stack.
#[derive(Component, Deref, DerefMut)]
pub struct PostprocessingPipelineId(CachedRenderPipelineId);

/// The on-GPU version of the [`ChromaticAberration`] settings.
///
/// See the documentation for [`ChromaticAberration`] for more information on
/// each of these fields.
#[derive(ShaderType)]
pub struct PostprocessingUniform {
    /// The intensity of the effect, in a fraction of the screen.
    chromatic_aberration_intensity: f32,
    /// A cap on the number of samples of the source texture that the shader
    /// will perform.
    chromatic_aberration_max_samples: u32,
    /// Padding data.
    unused_1: u32,
    /// Padding data.
    unused_2: u32,
}

/// A resource, part of the render world, that stores the
/// [`PostprocessingUniform`]s for each view.
#[derive(Resource, Deref, DerefMut, Default)]
pub struct PostprocessingUniformBuffer(DynamicUniformBuffer<PostprocessingUniform>);

/// A component, part of the render world, that stores the appropriate byte
/// offset within the [`PostprocessingUniformBuffer`] for the camera it's
/// attached to.
#[derive(Component, Deref, DerefMut)]
pub struct PostprocessingUniformBufferOffset(u32);

/// The render node that runs the built-in postprocessing stack.
#[derive(Default)]
pub struct PostprocessingNode;

impl Plugin for PostprocessingPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            POSTPROCESSING_SHADER_HANDLE,
            "postprocess.wgsl",
            Shader::from_wgsl
        );

        // Load the default chromatic aberration LUT.
        let mut assets = app.world_mut().resource_mut::<Assets<_>>();
        assets.insert(
            DEFAULT_CHROMATIC_ABERRATION_LUT_HANDLE.id(),
            Image::new(
                Extent3d {
                    width: 3,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D1,
                DEFAULT_CHROMATIC_ABBERATION_LUT_DATA.to_vec(),
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::RENDER_WORLD,
            ),
        );

        app.register_type::<ChromaticAberration>();
        app.add_plugins(ExtractComponentPlugin::<ChromaticAberration>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedRenderPipelines<PostprocessingPipeline>>()
            .init_resource::<PostprocessingUniformBuffer>()
            .add_systems(
                Render,
                (
                    prepare_postprocessing_pipelines,
                    prepare_postprocessing_uniforms,
                )
                    .in_set(RenderSet::Prepare),
            )
            .add_render_graph_node::<ViewNodeRunner<PostprocessingNode>>(
                Core3d,
                Node3d::Postprocessing,
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::DepthOfField,
                    Node3d::Postprocessing,
                    Node3d::Tonemapping,
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<PostprocessingNode>>(
                Core2d,
                Node2d::Postprocessing,
            )
            .add_render_graph_edges(
                Core2d,
                (Node2d::Bloom, Node2d::Postprocessing, Node2d::Tonemapping),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<PostprocessingPipeline>();
    }
}

impl Default for ChromaticAberration {
    fn default() -> Self {
        Self {
            color_lut: DEFAULT_CHROMATIC_ABERRATION_LUT_HANDLE,
            intensity: DEFAULT_CHROMATIC_ABBERATION_INTENSITY,
            max_samples: DEFAULT_CHROMATIC_ABERRATION_MAX_SAMPLES,
        }
    }
}

impl FromWorld for PostprocessingPipeline {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        // Create our single bind group layout.
        let bind_group_layout = render_device.create_bind_group_layout(
            Some("postprocessing bind group layout"),
            &BindGroupLayoutEntries::sequential(
                ShaderStages::FRAGMENT,
                (
                    // Source:
                    texture_2d(TextureSampleType::Float { filterable: true }),
                    // Source sampler:
                    sampler(SamplerBindingType::Filtering),
                    // Chromatic aberration LUT:
                    texture_1d(TextureSampleType::Float { filterable: true }),
                    // Chromatic aberration LUT sampler:
                    sampler(SamplerBindingType::Filtering),
                    // Postprocessing settings:
                    uniform_buffer::<PostprocessingUniform>(true),
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

        PostprocessingPipeline {
            bind_group_layout,
            source_sampler,
            chromatic_aberration_lut_sampler,
        }
    }
}

impl SpecializedRenderPipeline for PostprocessingPipeline {
    type Key = PostprocessingPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        RenderPipelineDescriptor {
            label: Some("postprocessing".into()),
            layout: vec![self.bind_group_layout.clone()],
            vertex: fullscreen_vertex_shader::fullscreen_shader_vertex_state(),
            fragment: Some(FragmentState {
                shader: POSTPROCESSING_SHADER_HANDLE,
                shader_defs: vec![],
                entry_point: "fragment_main".into(),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            primitive: default(),
            depth_stencil: None,
            multisample: default(),
            push_constant_ranges: vec![],
        }
    }
}

impl ViewNode for PostprocessingNode {
    type ViewQuery = (
        Read<ViewTarget>,
        Read<PostprocessingPipelineId>,
        Read<ChromaticAberration>,
        Read<PostprocessingUniformBufferOffset>,
    );

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (view_target, pipeline_id, chromatic_aberration, postprocessing_uniform_buffer_offset): QueryItem<'w, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let postprocessing_pipeline = world.resource::<PostprocessingPipeline>();
        let postprocessing_uniform_buffer = world.resource::<PostprocessingUniformBuffer>();
        let gpu_image_assets = world.resource::<RenderAssets<GpuImage>>();

        // We need a render pipeline to be prepared.
        let Some(pipeline) = pipeline_cache.get_render_pipeline(**pipeline_id) else {
            return Ok(());
        };

        // We need the chromatic aberration LUT to be present.
        let Some(chromatic_aberration_lut) = gpu_image_assets.get(&chromatic_aberration.color_lut)
        else {
            return Ok(());
        };

        // We need the postprocessing settings to be uploaded to the GPU.
        let Some(postprocessing_uniform_buffer_binding) = postprocessing_uniform_buffer.binding()
        else {
            return Ok(());
        };

        // Use the [`PostProcessWrite`] infrastructure, since this is a
        // full-screen pass.
        let postprocess = view_target.post_process_write();

        let pass_descriptor = RenderPassDescriptor {
            label: Some("postprocessing pass"),
            color_attachments: &[Some(RenderPassColorAttachment {
                view: postprocess.destination,
                resolve_target: None,
                ops: Operations::default(),
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        };

        let bind_group = render_context.render_device().create_bind_group(
            Some("postprocessing bind group"),
            &postprocessing_pipeline.bind_group_layout,
            &BindGroupEntries::sequential((
                postprocess.source,
                &postprocessing_pipeline.source_sampler,
                &chromatic_aberration_lut.texture_view,
                &postprocessing_pipeline.chromatic_aberration_lut_sampler,
                postprocessing_uniform_buffer_binding,
            )),
        );

        let mut render_pass = render_context
            .command_encoder()
            .begin_render_pass(&pass_descriptor);

        render_pass.set_pipeline(pipeline);
        render_pass.set_bind_group(0, &bind_group, &[**postprocessing_uniform_buffer_offset]);
        render_pass.draw(0..3, 0..1);

        Ok(())
    }
}

/// Specializes the built-in postprocessing pipeline for each applicable view.
pub fn prepare_postprocessing_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut pipelines: ResMut<SpecializedRenderPipelines<PostprocessingPipeline>>,
    postprocessing_pipeline: Res<PostprocessingPipeline>,
    views: Query<(Entity, &ExtractedView), With<ChromaticAberration>>,
) {
    for (entity, view) in views.iter() {
        let pipeline_id = pipelines.specialize(
            &pipeline_cache,
            &postprocessing_pipeline,
            PostprocessingPipelineKey {
                texture_format: if view.hdr {
                    ViewTarget::TEXTURE_FORMAT_HDR
                } else {
                    TextureFormat::bevy_default()
                },
            },
        );

        commands
            .entity(entity)
            .insert(PostprocessingPipelineId(pipeline_id));
    }
}

/// Gathers the built-in postprocessing settings for every view and uploads them
/// to the GPU.
pub fn prepare_postprocessing_uniforms(
    mut commands: Commands,
    mut postprocessing_uniform_buffer: ResMut<PostprocessingUniformBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut views: Query<(Entity, &ChromaticAberration)>,
) {
    postprocessing_uniform_buffer.clear();

    // Gather up all the postprocessing settings.
    for (view_entity, chromatic_aberration) in views.iter_mut() {
        let uniform_buffer_offset = postprocessing_uniform_buffer.push(&PostprocessingUniform {
            chromatic_aberration_intensity: chromatic_aberration.intensity,
            chromatic_aberration_max_samples: chromatic_aberration.max_samples,
            unused_1: 0,
            unused_2: 0,
        });
        commands
            .entity(view_entity)
            .insert(PostprocessingUniformBufferOffset(uniform_buffer_offset));
    }

    // Upload to the GPU.
    postprocessing_uniform_buffer.write_buffer(&render_device, &render_queue);
}

impl ExtractComponent for ChromaticAberration {
    type QueryData = Read<ChromaticAberration>;

    type QueryFilter = With<Camera>;

    type Out = ChromaticAberration;

    fn extract_component(
        chromatic_aberration: QueryItem<'_, Self::QueryData>,
    ) -> Option<Self::Out> {
        // Skip the postprocessing phase entirely if the intensity is zero.
        if chromatic_aberration.intensity > 0.0 {
            Some(chromatic_aberration.clone())
        } else {
            None
        }
    }
}
