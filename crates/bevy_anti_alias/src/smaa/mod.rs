//! Subpixel morphological antialiasing (SMAA).
//!
//! [SMAA] is a 2011 antialiasing technique that takes an aliased image and
//! smooths out the *jaggies*, making edges smoother. It's been used in numerous
//! games and has become a staple postprocessing technique. Compared to MSAA,
//! SMAA has the advantage of compatibility with deferred rendering and
//! reduction of GPU memory bandwidth.  Compared to FXAA, SMAA has the advantage
//! of improved quality, but the disadvantage of reduced performance. Compared
//! to TAA, SMAA has the advantage of stability and lack of *ghosting*
//! artifacts, but has the disadvantage of not supporting temporal accumulation,
//! which have made SMAA less popular when advanced photorealistic rendering
//! features are used in recent years.
//!
//! To use SMAA, add [`Smaa`] to a [`bevy_camera::Camera`]. In a
//! pinch, you can simply use the default settings (via the [`Default`] trait)
//! for a high-quality, high-performance appearance. When using SMAA, you will
//! likely want set [`bevy_render::view::Msaa`] to [`bevy_render::view::Msaa::Off`]
//! for every camera using SMAA.
//!
//! Those who have used SMAA in other engines should be aware that Bevy doesn't
//! yet support the following more advanced features of SMAA:
//!
//! * The temporal variant.
//!
//! * Depth- and chroma-based edge detection.
//!
//! * Predicated thresholding.
//!
//! * Compatibility with SSAA and MSAA.
//!
//! [SMAA]: https://www.iryoku.com/smaa/
use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetServer, Handle};
#[cfg(not(feature = "smaa_luts"))]
use bevy_core_pipeline::tonemapping::lut_placeholder;
use bevy_core_pipeline::{
    core_2d::graph::{Core2d, Node2d},
    core_3d::graph::{Core3d, Node3d},
};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::{QueryItem, With},
    reflect::ReflectComponent,
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{lifetimeless::Read, Commands, Query, Res, ResMut},
    world::World,
};
use bevy_image::{BevyDefault, Image, ToExtents};
use bevy_math::{vec4, Vec4};
use bevy_reflect::{std_traits::ReflectDefault, Reflect};
use bevy_render::{
    camera::ExtractedCamera,
    diagnostic::RecordDiagnostics,
    extract_component::{ExtractComponent, ExtractComponentPlugin},
    render_asset::RenderAssets,
    render_graph::{
        NodeRunError, RenderGraphContext, RenderGraphExt as _, ViewNode, ViewNodeRunner,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, uniform_buffer},
        AddressMode, BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutEntries,
        CachedRenderPipelineId, ColorTargetState, ColorWrites, CompareFunction, DepthStencilState,
        DynamicUniformBuffer, FilterMode, FragmentState, LoadOp, Operations, PipelineCache,
        RenderPassColorAttachment, RenderPassDepthStencilAttachment, RenderPassDescriptor,
        RenderPipeline, RenderPipelineDescriptor, SamplerBindingType, SamplerDescriptor,
        ShaderStages, ShaderType, SpecializedRenderPipeline, SpecializedRenderPipelines,
        StencilFaceState, StencilOperation, StencilState, StoreOp, TextureDescriptor,
        TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
        VertexState,
    },
    renderer::{RenderContext, RenderDevice, RenderQueue},
    texture::{CachedTexture, GpuImage, TextureCache},
    view::{ExtractedView, ViewTarget},
    Render, RenderApp, RenderStartup, RenderSystems,
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::prelude::default;

/// Adds support for subpixel morphological antialiasing, or SMAA.
#[derive(Default)]
pub struct SmaaPlugin;

/// A component for enabling Subpixel Morphological Anti-Aliasing (SMAA)
/// for a [`bevy_camera::Camera`].
#[derive(Clone, Copy, Default, Component, Reflect, ExtractComponent)]
#[reflect(Component, Default, Clone)]
#[doc(alias = "SubpixelMorphologicalAntiAliasing")]
pub struct Smaa {
    /// A predefined set of SMAA parameters: i.e. a quality level.
    ///
    /// Generally, you can leave this at its default level.
    pub preset: SmaaPreset,
}

/// A preset quality level for SMAA.
///
/// Higher values are slower but result in a higher-quality image.
///
/// The default value is *high*.
#[derive(Clone, Copy, Reflect, Default, PartialEq, Eq, Hash)]
#[reflect(Default, Clone, PartialEq, Hash)]
pub enum SmaaPreset {
    /// Four search steps; no diagonal or corner detection.
    Low,

    /// Eight search steps; no diagonal or corner detection.
    Medium,

    /// Sixteen search steps, 8 diagonal search steps, and corner detection.
    ///
    /// This is the default.
    #[default]
    High,

    /// Thirty-two search steps, 8 diagonal search steps, and corner detection.
    Ultra,
}

#[derive(Resource)]
struct SmaaLuts {
    /// The handle of the area LUT, a KTX2 format texture that SMAA uses internally.
    area_lut: Handle<Image>,
    /// The handle of the search LUT, a KTX2 format texture that SMAA uses internally.
    search_lut: Handle<Image>,
}

/// A render world resource that holds all render pipeline data needed for SMAA.
///
/// There are three separate passes, so we need three separate pipelines.
#[derive(Resource)]
pub struct SmaaPipelines {
    /// Pass 1: Edge detection.
    edge_detection: SmaaEdgeDetectionPipeline,
    /// Pass 2: Blending weight calculation.
    blending_weight_calculation: SmaaBlendingWeightCalculationPipeline,
    /// Pass 3: Neighborhood blending.
    neighborhood_blending: SmaaNeighborhoodBlendingPipeline,
}

/// The pipeline data for phase 1 of SMAA: edge detection.
struct SmaaEdgeDetectionPipeline {
    /// The bind group layout common to all passes.
    postprocess_bind_group_layout: BindGroupLayout,
    /// The bind group layout for data specific to this pass.
    edge_detection_bind_group_layout: BindGroupLayout,
    /// The shader asset handle.
    shader: Handle<Shader>,
}

/// The pipeline data for phase 2 of SMAA: blending weight calculation.
struct SmaaBlendingWeightCalculationPipeline {
    /// The bind group layout common to all passes.
    postprocess_bind_group_layout: BindGroupLayout,
    /// The bind group layout for data specific to this pass.
    blending_weight_calculation_bind_group_layout: BindGroupLayout,
    /// The shader asset handle.
    shader: Handle<Shader>,
}

/// The pipeline data for phase 3 of SMAA: neighborhood blending.
struct SmaaNeighborhoodBlendingPipeline {
    /// The bind group layout common to all passes.
    postprocess_bind_group_layout: BindGroupLayout,
    /// The bind group layout for data specific to this pass.
    neighborhood_blending_bind_group_layout: BindGroupLayout,
    /// The shader asset handle.
    shader: Handle<Shader>,
}

/// A unique identifier for a set of SMAA pipelines.
#[derive(Clone, PartialEq, Eq, Hash)]
pub struct SmaaNeighborhoodBlendingPipelineKey {
    /// The format of the framebuffer.
    texture_format: TextureFormat,
    /// The quality preset.
    preset: SmaaPreset,
}

/// A render world component that holds the pipeline IDs for the SMAA passes.
///
/// There are three separate SMAA passes, each with a different shader and bind
/// group layout, so we need three pipeline IDs.
#[derive(Component)]
pub struct ViewSmaaPipelines {
    /// The pipeline ID for edge detection (phase 1).
    edge_detection_pipeline_id: CachedRenderPipelineId,
    /// The pipeline ID for blending weight calculation (phase 2).
    blending_weight_calculation_pipeline_id: CachedRenderPipelineId,
    /// The pipeline ID for neighborhood blending (phase 3).
    neighborhood_blending_pipeline_id: CachedRenderPipelineId,
}

/// The render graph node that performs subpixel morphological antialiasing
/// (SMAA).
#[derive(Default)]
pub struct SmaaNode;

/// Values supplied to the GPU for SMAA.
///
/// Currently, this just contains the render target metrics and values derived
/// from them. These could be computed by the shader itself, but the original
/// SMAA HLSL code supplied them in a uniform, so we do the same for
/// consistency.
#[derive(Clone, Copy, ShaderType)]
pub struct SmaaInfoUniform {
    /// Information about the width and height of the framebuffer.
    ///
    /// * *x*: The reciprocal pixel width of the framebuffer.
    ///
    /// * *y*: The reciprocal pixel height of the framebuffer.
    ///
    /// * *z*: The pixel width of the framebuffer.
    ///
    /// * *w*: The pixel height of the framebuffer.
    pub rt_metrics: Vec4,
}

/// A render world component that stores the offset of each [`SmaaInfoUniform`]
/// within the [`SmaaInfoUniformBuffer`] for each view.
#[derive(Clone, Copy, Deref, DerefMut, Component)]
pub struct SmaaInfoUniformOffset(pub u32);

/// The GPU buffer that holds all [`SmaaInfoUniform`]s for all views.
///
/// This is a resource stored in the render world.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct SmaaInfoUniformBuffer(pub DynamicUniformBuffer<SmaaInfoUniform>);

/// A render world component that holds the intermediate textures necessary to
/// perform SMAA.
///
/// This is stored on each view that has enabled SMAA.
#[derive(Component)]
pub struct SmaaTextures {
    /// The two-channel texture that stores the output from the first pass (edge
    /// detection).
    ///
    /// The second pass (blending weight calculation) reads this texture to do
    /// its work.
    pub edge_detection_color_texture: CachedTexture,

    /// The 8-bit stencil texture that records which pixels the first pass
    /// touched, so that the second pass doesn't have to examine other pixels.
    ///
    /// Each texel will contain a 0 if the first pass didn't touch the
    /// corresponding pixel or a 1 if the first pass did touch that pixel.
    pub edge_detection_stencil_texture: CachedTexture,

    /// A four-channel RGBA texture that stores the output from the second pass
    /// (blending weight calculation).
    ///
    /// The final pass (neighborhood blending) reads this texture to do its
    /// work.
    pub blend_texture: CachedTexture,
}

/// A render world component that stores the bind groups necessary to perform
/// SMAA.
///
/// This is stored on each view.
#[derive(Component)]
pub struct SmaaBindGroups {
    /// The bind group for the first pass (edge detection).
    pub edge_detection_bind_group: BindGroup,
    /// The bind group for the second pass (blending weight calculation).
    pub blending_weight_calculation_bind_group: BindGroup,
    /// The bind group for the final pass (neighborhood blending).
    pub neighborhood_blending_bind_group: BindGroup,
}

/// Stores the specialized render pipelines for SMAA.
///
/// Because SMAA uses three passes, we need three separate render pipeline
/// stores.
#[derive(Resource, Default)]
pub struct SmaaSpecializedRenderPipelines {
    /// Specialized render pipelines for the first phase (edge detection).
    edge_detection: SpecializedRenderPipelines<SmaaEdgeDetectionPipeline>,

    /// Specialized render pipelines for the second phase (blending weight
    /// calculation).
    blending_weight_calculation: SpecializedRenderPipelines<SmaaBlendingWeightCalculationPipeline>,

    /// Specialized render pipelines for the third phase (neighborhood
    /// blending).
    neighborhood_blending: SpecializedRenderPipelines<SmaaNeighborhoodBlendingPipeline>,
}

impl Plugin for SmaaPlugin {
    fn build(&self, app: &mut App) {
        // Load the shader.
        embedded_asset!(app, "smaa.wgsl");

        #[cfg(feature = "smaa_luts")]
        let smaa_luts = {
            // Load the two lookup textures. These are compressed textures in KTX2 format.
            embedded_asset!(app, "SMAAAreaLUT.ktx2");
            embedded_asset!(app, "SMAASearchLUT.ktx2");

            SmaaLuts {
                area_lut: load_embedded_asset!(app, "SMAAAreaLUT.ktx2"),
                search_lut: load_embedded_asset!(app, "SMAASearchLUT.ktx2"),
            }
        };
        #[cfg(not(feature = "smaa_luts"))]
        let smaa_luts = {
            let mut images = app.world_mut().resource_mut::<bevy_asset::Assets<Image>>();
            let handle = images.add(lut_placeholder());
            SmaaLuts {
                area_lut: handle.clone(),
                search_lut: handle.clone(),
            }
        };

        app.add_plugins(ExtractComponentPlugin::<Smaa>::default());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .insert_resource(smaa_luts)
            .init_resource::<SmaaSpecializedRenderPipelines>()
            .init_resource::<SmaaInfoUniformBuffer>()
            .add_systems(RenderStartup, init_smaa_pipelines)
            .add_systems(
                Render,
                (
                    prepare_smaa_pipelines.in_set(RenderSystems::Prepare),
                    prepare_smaa_uniforms.in_set(RenderSystems::PrepareResources),
                    prepare_smaa_textures.in_set(RenderSystems::PrepareResources),
                    prepare_smaa_bind_groups.in_set(RenderSystems::PrepareBindGroups),
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<SmaaNode>>(Core3d, Node3d::Smaa)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::Tonemapping,
                    Node3d::Smaa,
                    Node3d::EndMainPassPostProcessing,
                ),
            )
            .add_render_graph_node::<ViewNodeRunner<SmaaNode>>(Core2d, Node2d::Smaa)
            .add_render_graph_edges(
                Core2d,
                (
                    Node2d::Tonemapping,
                    Node2d::Smaa,
                    Node2d::EndMainPassPostProcessing,
                ),
            );
    }
}

pub fn init_smaa_pipelines(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    asset_server: Res<AssetServer>,
) {
    // Create the postprocess bind group layout (all passes, bind group 0).
    let postprocess_bind_group_layout = render_device.create_bind_group_layout(
        "SMAA postprocess bind group layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                uniform_buffer::<SmaaInfoUniform>(true).visibility(ShaderStages::VERTEX_FRAGMENT),
            ),
        ),
    );

    // Create the edge detection bind group layout (pass 1, bind group 1).
    let edge_detection_bind_group_layout = render_device.create_bind_group_layout(
        "SMAA edge detection bind group layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (sampler(SamplerBindingType::Filtering),),
        ),
    );

    // Create the blending weight calculation bind group layout (pass 2, bind group 1).
    let blending_weight_calculation_bind_group_layout = render_device.create_bind_group_layout(
        "SMAA blending weight calculation bind group layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }), // edges texture
                sampler(SamplerBindingType::Filtering),                    // edges sampler
                texture_2d(TextureSampleType::Float { filterable: true }), // search texture
                texture_2d(TextureSampleType::Float { filterable: true }), // area texture
            ),
        ),
    );

    // Create the neighborhood blending bind group layout (pass 3, bind group 1).
    let neighborhood_blending_bind_group_layout = render_device.create_bind_group_layout(
        "SMAA neighborhood blending bind group layout",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::FRAGMENT,
            (
                texture_2d(TextureSampleType::Float { filterable: true }),
                sampler(SamplerBindingType::Filtering),
            ),
        ),
    );

    let shader = load_embedded_asset!(asset_server.as_ref(), "smaa.wgsl");

    commands.insert_resource(SmaaPipelines {
        edge_detection: SmaaEdgeDetectionPipeline {
            postprocess_bind_group_layout: postprocess_bind_group_layout.clone(),
            edge_detection_bind_group_layout,
            shader: shader.clone(),
        },
        blending_weight_calculation: SmaaBlendingWeightCalculationPipeline {
            postprocess_bind_group_layout: postprocess_bind_group_layout.clone(),
            blending_weight_calculation_bind_group_layout,
            shader: shader.clone(),
        },
        neighborhood_blending: SmaaNeighborhoodBlendingPipeline {
            postprocess_bind_group_layout,
            neighborhood_blending_bind_group_layout,
            shader,
        },
    });
}

// Phase 1: edge detection.
impl SpecializedRenderPipeline for SmaaEdgeDetectionPipeline {
    type Key = SmaaPreset;

    fn specialize(&self, preset: Self::Key) -> RenderPipelineDescriptor {
        let shader_defs = vec!["SMAA_EDGE_DETECTION".into(), preset.shader_def()];

        // We mark the pixels that we touched with a 1 so that the blending
        // weight calculation (phase 2) will only consider those. This reduces
        // the overhead of phase 2 considerably.
        let stencil_face_state = StencilFaceState {
            compare: CompareFunction::Always,
            fail_op: StencilOperation::Replace,
            depth_fail_op: StencilOperation::Replace,
            pass_op: StencilOperation::Replace,
        };

        RenderPipelineDescriptor {
            label: Some("SMAA edge detection".into()),
            layout: vec![
                self.postprocess_bind_group_layout.clone(),
                self.edge_detection_bind_group_layout.clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                entry_point: Some("edge_detection_vertex_main".into()),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: Some("luma_edge_detection_fragment_main".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rg8Unorm,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Stencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: stencil_face_state,
                    back: stencil_face_state,
                    read_mask: 1,
                    write_mask: 1,
                },
                bias: default(),
            }),
            ..default()
        }
    }
}

// Phase 2: blending weight calculation.
impl SpecializedRenderPipeline for SmaaBlendingWeightCalculationPipeline {
    type Key = SmaaPreset;

    fn specialize(&self, preset: Self::Key) -> RenderPipelineDescriptor {
        let shader_defs = vec![
            "SMAA_BLENDING_WEIGHT_CALCULATION".into(),
            preset.shader_def(),
        ];

        // Only consider the pixels that were touched in phase 1.
        let stencil_face_state = StencilFaceState {
            compare: CompareFunction::Equal,
            fail_op: StencilOperation::Keep,
            depth_fail_op: StencilOperation::Keep,
            pass_op: StencilOperation::Keep,
        };

        RenderPipelineDescriptor {
            label: Some("SMAA blending weight calculation".into()),
            layout: vec![
                self.postprocess_bind_group_layout.clone(),
                self.blending_weight_calculation_bind_group_layout.clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                entry_point: Some("blending_weight_calculation_vertex_main".into()),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: Some("blending_weight_calculation_fragment_main".into()),
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba8Unorm,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Stencil8,
                depth_write_enabled: false,
                depth_compare: CompareFunction::Always,
                stencil: StencilState {
                    front: stencil_face_state,
                    back: stencil_face_state,
                    read_mask: 1,
                    write_mask: 1,
                },
                bias: default(),
            }),
            ..default()
        }
    }
}

impl SpecializedRenderPipeline for SmaaNeighborhoodBlendingPipeline {
    type Key = SmaaNeighborhoodBlendingPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let shader_defs = vec!["SMAA_NEIGHBORHOOD_BLENDING".into(), key.preset.shader_def()];

        RenderPipelineDescriptor {
            label: Some("SMAA neighborhood blending".into()),
            layout: vec![
                self.postprocess_bind_group_layout.clone(),
                self.neighborhood_blending_bind_group_layout.clone(),
            ],
            vertex: VertexState {
                shader: self.shader.clone(),
                shader_defs: shader_defs.clone(),
                entry_point: Some("neighborhood_blending_vertex_main".into()),
                buffers: vec![],
            },
            fragment: Some(FragmentState {
                shader: self.shader.clone(),
                shader_defs,
                entry_point: Some("neighborhood_blending_fragment_main".into()),
                targets: vec![Some(ColorTargetState {
                    format: key.texture_format,
                    blend: None,
                    write_mask: ColorWrites::ALL,
                })],
            }),
            ..default()
        }
    }
}

/// A system, part of the render app, that specializes the three pipelines
/// needed for SMAA according to each view's SMAA settings.
fn prepare_smaa_pipelines(
    mut commands: Commands,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_render_pipelines: ResMut<SmaaSpecializedRenderPipelines>,
    smaa_pipelines: Res<SmaaPipelines>,
    view_targets: Query<(Entity, &ExtractedView, &Smaa)>,
) {
    for (entity, view, smaa) in &view_targets {
        let edge_detection_pipeline_id = specialized_render_pipelines.edge_detection.specialize(
            &pipeline_cache,
            &smaa_pipelines.edge_detection,
            smaa.preset,
        );

        let blending_weight_calculation_pipeline_id = specialized_render_pipelines
            .blending_weight_calculation
            .specialize(
                &pipeline_cache,
                &smaa_pipelines.blending_weight_calculation,
                smaa.preset,
            );

        let neighborhood_blending_pipeline_id = specialized_render_pipelines
            .neighborhood_blending
            .specialize(
                &pipeline_cache,
                &smaa_pipelines.neighborhood_blending,
                SmaaNeighborhoodBlendingPipelineKey {
                    texture_format: if view.hdr {
                        ViewTarget::TEXTURE_FORMAT_HDR
                    } else {
                        TextureFormat::bevy_default()
                    },
                    preset: smaa.preset,
                },
            );

        commands.entity(entity).insert(ViewSmaaPipelines {
            edge_detection_pipeline_id,
            blending_weight_calculation_pipeline_id,
            neighborhood_blending_pipeline_id,
        });
    }
}

/// A system, part of the render app, that builds the [`SmaaInfoUniform`] data
/// for each view with SMAA enabled and writes the resulting data to GPU memory.
fn prepare_smaa_uniforms(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    view_targets: Query<(Entity, &ExtractedView), With<Smaa>>,
    mut smaa_info_buffer: ResMut<SmaaInfoUniformBuffer>,
) {
    smaa_info_buffer.clear();
    for (entity, view) in &view_targets {
        let offset = smaa_info_buffer.push(&SmaaInfoUniform {
            rt_metrics: vec4(
                1.0 / view.viewport.z as f32,
                1.0 / view.viewport.w as f32,
                view.viewport.z as f32,
                view.viewport.w as f32,
            ),
        });
        commands
            .entity(entity)
            .insert(SmaaInfoUniformOffset(offset));
    }

    smaa_info_buffer.write_buffer(&render_device, &render_queue);
}

/// A system, part of the render app, that builds the intermediate textures for
/// each view with SMAA enabled.
///
/// Phase 1 (edge detection) needs a two-channel RG texture and an 8-bit stencil
/// texture; phase 2 (blend weight calculation) needs a four-channel RGBA
/// texture.
fn prepare_smaa_textures(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    view_targets: Query<(Entity, &ExtractedCamera), (With<ExtractedView>, With<Smaa>)>,
) {
    for (entity, camera) in &view_targets {
        let Some(texture_size) = camera.physical_target_size else {
            continue;
        };

        // Create the two-channel RG texture for phase 1 (edge detection).
        let edge_detection_color_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("SMAA edge detection color texture"),
                size: texture_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rg8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
        );

        // Create the stencil texture for phase 1 (edge detection).
        let edge_detection_stencil_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("SMAA edge detection stencil texture"),
                size: texture_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Stencil8,
                usage: TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
        );

        // Create the four-channel RGBA texture for phase 2 (blending weight
        // calculation).
        let blend_texture = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("SMAA blend texture"),
                size: texture_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::RENDER_ATTACHMENT,
                view_formats: &[],
            },
        );

        commands.entity(entity).insert(SmaaTextures {
            edge_detection_color_texture,
            edge_detection_stencil_texture,
            blend_texture,
        });
    }
}

/// A system, part of the render app, that builds the SMAA bind groups for each
/// view with SMAA enabled.
fn prepare_smaa_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    smaa_pipelines: Res<SmaaPipelines>,
    smaa_luts: Res<SmaaLuts>,
    images: Res<RenderAssets<GpuImage>>,
    view_targets: Query<(Entity, &SmaaTextures), (With<ExtractedView>, With<Smaa>)>,
) {
    // Fetch the two lookup textures. These are bundled in this library.
    let (Some(search_texture), Some(area_texture)) = (
        images.get(&smaa_luts.search_lut),
        images.get(&smaa_luts.area_lut),
    ) else {
        return;
    };

    for (entity, smaa_textures) in &view_targets {
        // We use the same sampler settings for all textures, so we can build
        // only one and reuse it.
        let sampler = render_device.create_sampler(&SamplerDescriptor {
            label: Some("SMAA sampler"),
            address_mode_u: AddressMode::ClampToEdge,
            address_mode_v: AddressMode::ClampToEdge,
            address_mode_w: AddressMode::ClampToEdge,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            ..default()
        });

        commands.entity(entity).insert(SmaaBindGroups {
            edge_detection_bind_group: render_device.create_bind_group(
                Some("SMAA edge detection bind group"),
                &smaa_pipelines
                    .edge_detection
                    .edge_detection_bind_group_layout,
                &BindGroupEntries::sequential((&sampler,)),
            ),
            blending_weight_calculation_bind_group: render_device.create_bind_group(
                Some("SMAA blending weight calculation bind group"),
                &smaa_pipelines
                    .blending_weight_calculation
                    .blending_weight_calculation_bind_group_layout,
                &BindGroupEntries::sequential((
                    &smaa_textures.edge_detection_color_texture.default_view,
                    &sampler,
                    &search_texture.texture_view,
                    &area_texture.texture_view,
                )),
            ),
            neighborhood_blending_bind_group: render_device.create_bind_group(
                Some("SMAA neighborhood blending bind group"),
                &smaa_pipelines
                    .neighborhood_blending
                    .neighborhood_blending_bind_group_layout,
                &BindGroupEntries::sequential((
                    &smaa_textures.blend_texture.default_view,
                    &sampler,
                )),
            ),
        });
    }
}

impl ViewNode for SmaaNode {
    type ViewQuery = (
        Read<ViewTarget>,
        Read<ViewSmaaPipelines>,
        Read<SmaaInfoUniformOffset>,
        Read<SmaaTextures>,
        Read<SmaaBindGroups>,
    );

    fn run<'w>(
        &self,
        _: &mut RenderGraphContext,
        render_context: &mut RenderContext<'w>,
        (
            view_target,
            view_pipelines,
            view_smaa_uniform_offset,
            smaa_textures,
            view_smaa_bind_groups,
        ): QueryItem<'w, '_, Self::ViewQuery>,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let pipeline_cache = world.resource::<PipelineCache>();
        let smaa_pipelines = world.resource::<SmaaPipelines>();
        let smaa_info_uniform_buffer = world.resource::<SmaaInfoUniformBuffer>();

        // Fetch the render pipelines.
        let (
            Some(edge_detection_pipeline),
            Some(blending_weight_calculation_pipeline),
            Some(neighborhood_blending_pipeline),
        ) = (
            pipeline_cache.get_render_pipeline(view_pipelines.edge_detection_pipeline_id),
            pipeline_cache
                .get_render_pipeline(view_pipelines.blending_weight_calculation_pipeline_id),
            pipeline_cache.get_render_pipeline(view_pipelines.neighborhood_blending_pipeline_id),
        )
        else {
            return Ok(());
        };

        let diagnostics = render_context.diagnostic_recorder();
        render_context.command_encoder().push_debug_group("smaa");
        let time_span = diagnostics.time_span(render_context.command_encoder(), "smaa");

        // Fetch the framebuffer textures.
        let postprocess = view_target.post_process_write();
        let (source, destination) = (postprocess.source, postprocess.destination);

        // Stage 1: Edge detection pass.
        perform_edge_detection(
            render_context,
            smaa_pipelines,
            smaa_textures,
            view_smaa_bind_groups,
            smaa_info_uniform_buffer,
            view_smaa_uniform_offset,
            edge_detection_pipeline,
            source,
        );

        // Stage 2: Blending weight calculation pass.
        perform_blending_weight_calculation(
            render_context,
            smaa_pipelines,
            smaa_textures,
            view_smaa_bind_groups,
            smaa_info_uniform_buffer,
            view_smaa_uniform_offset,
            blending_weight_calculation_pipeline,
            source,
        );

        // Stage 3: Neighborhood blending pass.
        perform_neighborhood_blending(
            render_context,
            smaa_pipelines,
            view_smaa_bind_groups,
            smaa_info_uniform_buffer,
            view_smaa_uniform_offset,
            neighborhood_blending_pipeline,
            source,
            destination,
        );

        time_span.end(render_context.command_encoder());
        render_context.command_encoder().pop_debug_group();

        Ok(())
    }
}

/// Performs edge detection (phase 1).
///
/// This runs as part of the [`SmaaNode`]. It reads from the source texture and
/// writes to the two-channel RG edges texture. Additionally, it ensures that
/// all pixels it didn't touch are stenciled out so that phase 2 won't have to
/// examine them.
fn perform_edge_detection(
    render_context: &mut RenderContext,
    smaa_pipelines: &SmaaPipelines,
    smaa_textures: &SmaaTextures,
    view_smaa_bind_groups: &SmaaBindGroups,
    smaa_info_uniform_buffer: &SmaaInfoUniformBuffer,
    view_smaa_uniform_offset: &SmaaInfoUniformOffset,
    edge_detection_pipeline: &RenderPipeline,
    source: &TextureView,
) {
    // Create the edge detection bind group.
    let postprocess_bind_group = render_context.render_device().create_bind_group(
        None,
        &smaa_pipelines.edge_detection.postprocess_bind_group_layout,
        &BindGroupEntries::sequential((source, &**smaa_info_uniform_buffer)),
    );

    // Create the edge detection pass descriptor.
    let pass_descriptor = RenderPassDescriptor {
        label: Some("SMAA edge detection pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &smaa_textures.edge_detection_color_texture.default_view,
            depth_slice: None,
            resolve_target: None,
            ops: default(),
        })],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &smaa_textures.edge_detection_stencil_texture.default_view,
            depth_ops: None,
            stencil_ops: Some(Operations {
                load: LoadOp::Clear(0),
                store: StoreOp::Store,
            }),
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    };

    // Run the actual render pass.
    let mut render_pass = render_context
        .command_encoder()
        .begin_render_pass(&pass_descriptor);
    render_pass.set_pipeline(edge_detection_pipeline);
    render_pass.set_bind_group(0, &postprocess_bind_group, &[**view_smaa_uniform_offset]);
    render_pass.set_bind_group(1, &view_smaa_bind_groups.edge_detection_bind_group, &[]);
    render_pass.set_stencil_reference(1);
    render_pass.draw(0..3, 0..1);
}

/// Performs blending weight calculation (phase 2).
///
/// This runs as part of the [`SmaaNode`]. It reads the edges texture and writes
/// to the blend weight texture, using the stencil buffer to avoid processing
/// pixels it doesn't need to examine.
fn perform_blending_weight_calculation(
    render_context: &mut RenderContext,
    smaa_pipelines: &SmaaPipelines,
    smaa_textures: &SmaaTextures,
    view_smaa_bind_groups: &SmaaBindGroups,
    smaa_info_uniform_buffer: &SmaaInfoUniformBuffer,
    view_smaa_uniform_offset: &SmaaInfoUniformOffset,
    blending_weight_calculation_pipeline: &RenderPipeline,
    source: &TextureView,
) {
    // Create the blending weight calculation bind group.
    let postprocess_bind_group = render_context.render_device().create_bind_group(
        None,
        &smaa_pipelines
            .blending_weight_calculation
            .postprocess_bind_group_layout,
        &BindGroupEntries::sequential((source, &**smaa_info_uniform_buffer)),
    );

    // Create the blending weight calculation pass descriptor.
    let pass_descriptor = RenderPassDescriptor {
        label: Some("SMAA blending weight calculation pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: &smaa_textures.blend_texture.default_view,
            depth_slice: None,
            resolve_target: None,
            ops: default(),
        })],
        depth_stencil_attachment: Some(RenderPassDepthStencilAttachment {
            view: &smaa_textures.edge_detection_stencil_texture.default_view,
            depth_ops: None,
            stencil_ops: Some(Operations {
                load: LoadOp::Load,
                store: StoreOp::Discard,
            }),
        }),
        timestamp_writes: None,
        occlusion_query_set: None,
    };

    // Run the actual render pass.
    let mut render_pass = render_context
        .command_encoder()
        .begin_render_pass(&pass_descriptor);
    render_pass.set_pipeline(blending_weight_calculation_pipeline);
    render_pass.set_bind_group(0, &postprocess_bind_group, &[**view_smaa_uniform_offset]);
    render_pass.set_bind_group(
        1,
        &view_smaa_bind_groups.blending_weight_calculation_bind_group,
        &[],
    );
    render_pass.set_stencil_reference(1);
    render_pass.draw(0..3, 0..1);
}

/// Performs blending weight calculation (phase 3).
///
/// This runs as part of the [`SmaaNode`]. It reads from the blend weight
/// texture. It's the only phase that writes to the postprocessing destination.
fn perform_neighborhood_blending(
    render_context: &mut RenderContext,
    smaa_pipelines: &SmaaPipelines,
    view_smaa_bind_groups: &SmaaBindGroups,
    smaa_info_uniform_buffer: &SmaaInfoUniformBuffer,
    view_smaa_uniform_offset: &SmaaInfoUniformOffset,
    neighborhood_blending_pipeline: &RenderPipeline,
    source: &TextureView,
    destination: &TextureView,
) {
    let postprocess_bind_group = render_context.render_device().create_bind_group(
        None,
        &smaa_pipelines
            .neighborhood_blending
            .postprocess_bind_group_layout,
        &BindGroupEntries::sequential((source, &**smaa_info_uniform_buffer)),
    );

    let pass_descriptor = RenderPassDescriptor {
        label: Some("SMAA neighborhood blending pass"),
        color_attachments: &[Some(RenderPassColorAttachment {
            view: destination,
            depth_slice: None,
            resolve_target: None,
            ops: default(),
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
    };

    let mut neighborhood_blending_render_pass = render_context
        .command_encoder()
        .begin_render_pass(&pass_descriptor);
    neighborhood_blending_render_pass.set_pipeline(neighborhood_blending_pipeline);
    neighborhood_blending_render_pass.set_bind_group(
        0,
        &postprocess_bind_group,
        &[**view_smaa_uniform_offset],
    );
    neighborhood_blending_render_pass.set_bind_group(
        1,
        &view_smaa_bind_groups.neighborhood_blending_bind_group,
        &[],
    );
    neighborhood_blending_render_pass.draw(0..3, 0..1);
}

impl SmaaPreset {
    /// Returns the `#define` in the shader corresponding to this quality
    /// preset.
    fn shader_def(&self) -> ShaderDefVal {
        match *self {
            SmaaPreset::Low => "SMAA_PRESET_LOW".into(),
            SmaaPreset::Medium => "SMAA_PRESET_MEDIUM".into(),
            SmaaPreset::High => "SMAA_PRESET_HIGH".into(),
            SmaaPreset::Ultra => "SMAA_PRESET_ULTRA".into(),
        }
    }
}
