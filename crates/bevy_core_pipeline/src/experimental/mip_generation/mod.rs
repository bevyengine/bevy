//! Downsampling of textures to produce mipmap levels.
//!
//! Currently, this module only supports generation of hierarchical Z buffers
//! for occlusion culling. It's marked experimental because the shader is
//! designed only for power-of-two texture sizes and is slightly incorrect for
//! non-power-of-two depth buffer sizes.

use core::array;

use crate::core_3d::{
    graph::{Core3d, Node3d},
    prepare_core_3d_depth_textures,
};
use bevy_app::{App, Plugin};
use bevy_asset::{load_internal_asset, weak_handle, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::{resource_exists, Without},
    query::{Or, QueryState, With},
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{lifetimeless::Read, Commands, Local, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_math::{uvec2, UVec2, Vec4Swizzles as _};
use bevy_render::{
    batching::gpu_preprocessing::GpuPreprocessingSupport,
    frame_graph::{
        BindGroupHandle, BindGroupResourceHandle, FrameGraph, ResourceMeta, TextureInfo,
        TextureViewInfo, TextureViewMeta,
    },
};
use bevy_render::{
    experimental::occlusion_culling::{
        OcclusionCulling, OcclusionCullingSubview, OcclusionCullingSubviewEntities,
    },
    render_graph::{Node, NodeRunError, RenderGraphApp, RenderGraphContext},
    render_resource::{
        binding_types::{sampler, texture_2d, texture_2d_multisampled, texture_storage_2d},
        BindGroupLayout, BindGroupLayoutEntries, CachedComputePipelineId,
        ComputePipelineDescriptor, Extent3d, PipelineCache, PushConstantRange, Sampler,
        SamplerBindingType, SamplerDescriptor, Shader, ShaderStages, SpecializedComputePipeline,
        SpecializedComputePipelines, StorageTextureAccess, TextureAspect, TextureDimension,
        TextureFormat, TextureSampleType, TextureUsages, TextureViewDimension,
    },
    renderer::RenderDevice,
    view::{ExtractedView, NoIndirectDrawing, ViewDepthTexture},
    Render, RenderApp, RenderSet,
};
use bitflags::bitflags;
use tracing::debug;

/// Identifies the `downsample_depth.wgsl` shader.
pub const DOWNSAMPLE_DEPTH_SHADER_HANDLE: Handle<Shader> =
    weak_handle!("a09a149e-5922-4fa4-9170-3c1a13065364");

/// The maximum number of mip levels that we can produce.
///
/// 2^12 is 4096, so that's the maximum size of the depth buffer that we
/// support.
pub const DEPTH_PYRAMID_MIP_COUNT: usize = 12;

/// A plugin that allows Bevy to repeatedly downsample textures to create
/// mipmaps.
///
/// Currently, this is only used for hierarchical Z buffer generation for the
/// purposes of occlusion culling.
pub struct MipGenerationPlugin;

impl Plugin for MipGenerationPlugin {
    fn build(&self, app: &mut App) {
        load_internal_asset!(
            app,
            DOWNSAMPLE_DEPTH_SHADER_HANDLE,
            "downsample_depth.wgsl",
            Shader::from_wgsl
        );

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedComputePipelines<DownsampleDepthPipeline>>()
            .add_render_graph_node::<DownsampleDepthNode>(Core3d, Node3d::EarlyDownsampleDepth)
            .add_render_graph_node::<DownsampleDepthNode>(Core3d, Node3d::LateDownsampleDepth)
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EarlyPrepass,
                    Node3d::EarlyDeferredPrepass,
                    Node3d::EarlyDownsampleDepth,
                    Node3d::LatePrepass,
                    Node3d::LateDeferredPrepass,
                ),
            )
            .add_render_graph_edges(
                Core3d,
                (
                    Node3d::EndMainPass,
                    Node3d::LateDownsampleDepth,
                    Node3d::EndMainPassPostProcessing,
                ),
            )
            .add_systems(
                Render,
                create_downsample_depth_pipelines.in_set(RenderSet::Prepare),
            )
            .add_systems(
                Render,
                (
                    prepare_view_depth_pyramids,
                    prepare_downsample_depth_view_bind_groups,
                )
                    .chain()
                    .in_set(RenderSet::PrepareResources)
                    .run_if(resource_exists::<DownsampleDepthPipelines>)
                    .after(prepare_core_3d_depth_textures),
            );
    }

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };
        render_app.init_resource::<DepthPyramidDummyTexture>();
    }
}

/// The nodes that produce a hierarchical Z-buffer, also known as a depth
/// pyramid.
///
/// This runs the single-pass downsampling (SPD) shader with the *min* filter in
/// order to generate a series of mipmaps for the Z buffer. The resulting
/// hierarchical Z-buffer can be used for occlusion culling.
///
/// There are two instances of this node. The *early* downsample depth pass is
/// the first hierarchical Z-buffer stage, which runs after the early prepass
/// and before the late prepass. It prepares the Z-buffer for the bounding box
/// tests that the late mesh preprocessing stage will perform. The *late*
/// downsample depth pass runs at the end of the main phase. It prepares the
/// Z-buffer for the occlusion culling that the early mesh preprocessing phase
/// of the *next* frame will perform.
///
/// This node won't do anything if occlusion culling isn't on.
pub struct DownsampleDepthNode {
    /// The query that we use to find views that need occlusion culling for
    /// their Z-buffer.
    main_view_query: QueryState<(
        Read<ViewDepthPyramid>,
        Read<ViewDownsampleDepthBindGroup>,
        Read<ViewDepthTexture>,
        Option<Read<OcclusionCullingSubviewEntities>>,
    )>,
    /// The query that we use to find shadow maps that need occlusion culling.
    shadow_view_query: QueryState<(
        Read<ViewDepthPyramid>,
        Read<ViewDownsampleDepthBindGroup>,
        Read<OcclusionCullingSubview>,
    )>,
}

impl FromWorld for DownsampleDepthNode {
    fn from_world(world: &mut World) -> Self {
        Self {
            main_view_query: QueryState::new(world),
            shadow_view_query: QueryState::new(world),
        }
    }
}

impl Node for DownsampleDepthNode {
    fn update(&mut self, world: &mut World) {
        self.main_view_query.update_archetypes(world);
        self.shadow_view_query.update_archetypes(world);
    }

    fn run<'w>(
        &self,
        render_graph_context: &mut RenderGraphContext,
        frame_graph: &mut FrameGraph,
        world: &'w World,
    ) -> Result<(), NodeRunError> {
        let Ok((
            view_depth_pyramid,
            view_downsample_depth_bind_group,
            view_depth_texture,
            maybe_view_light_entities,
        )) = self
            .main_view_query
            .get_manual(world, render_graph_context.view_entity())
        else {
            return Ok(());
        };

        // Downsample depth for the main Z-buffer.
        downsample_depth(
            render_graph_context,
            frame_graph,
            world,
            view_depth_pyramid,
            view_downsample_depth_bind_group,
            uvec2(
                view_depth_texture.texture.desc.size.width,
                view_depth_texture.texture.desc.size.height,
            ),
            view_depth_texture.texture.desc.sample_count,
        )?;

        // Downsample depth for shadow maps that have occlusion culling enabled.
        if let Some(view_light_entities) = maybe_view_light_entities {
            for &view_light_entity in &view_light_entities.0 {
                let Ok((view_depth_pyramid, view_downsample_depth_bind_group, occlusion_culling)) =
                    self.shadow_view_query.get_manual(world, view_light_entity)
                else {
                    continue;
                };
                downsample_depth(
                    render_graph_context,
                    frame_graph,
                    world,
                    view_depth_pyramid,
                    view_downsample_depth_bind_group,
                    UVec2::splat(occlusion_culling.depth_texture_size),
                    1,
                )?;
            }
        }

        Ok(())
    }
}

/// Produces a depth pyramid from the current depth buffer for a single view.
/// The resulting depth pyramid can be used for occlusion testing.
fn downsample_depth<'w>(
    render_graph_context: &mut RenderGraphContext,
    frame_graph: &mut FrameGraph,
    world: &'w World,
    view_depth_pyramid: &ViewDepthPyramid,
    view_downsample_depth_bind_group: &ViewDownsampleDepthBindGroup,
    view_size: UVec2,
    sample_count: u32,
) -> Result<(), NodeRunError> {
    let downsample_depth_pipelines = world.resource::<DownsampleDepthPipelines>();
    let pipeline_cache = world.resource::<PipelineCache>();

    // Despite the name "single-pass downsampling", we actually need two
    // passes because of the lack of `coherent` buffers in WGPU/WGSL.
    // Between each pass, there's an implicit synchronization barrier.

    // Fetch the appropriate pipeline ID, depending on whether the depth
    // buffer is multisampled or not.
    let (Some(first_downsample_depth_pipeline_id), Some(second_downsample_depth_pipeline_id)) =
        (if sample_count > 1 {
            (
                downsample_depth_pipelines.first_multisample.pipeline_id,
                downsample_depth_pipelines.second_multisample.pipeline_id,
            )
        } else {
            (
                downsample_depth_pipelines.first.pipeline_id,
                downsample_depth_pipelines.second.pipeline_id,
            )
        })
    else {
        return Ok(());
    };

    // Fetch the pipelines for the two passes.
    let (Some(_), Some(_)) = (
        pipeline_cache.get_compute_pipeline(first_downsample_depth_pipeline_id),
        pipeline_cache.get_compute_pipeline(second_downsample_depth_pipeline_id),
    ) else {
        return Ok(());
    };

    // Run the depth downsampling.
    view_depth_pyramid.downsample_depth(
        &format!("{:?}", render_graph_context.label()),
        frame_graph,
        view_size,
        view_downsample_depth_bind_group,
        first_downsample_depth_pipeline_id,
        second_downsample_depth_pipeline_id,
    );
    Ok(())
}

/// A single depth downsample pipeline.
#[derive(Resource)]
pub struct DownsampleDepthPipeline {
    /// The bind group layout for this pipeline.
    bind_group_layout: BindGroupLayout,
    /// A handle that identifies the compiled shader.
    pipeline_id: Option<CachedComputePipelineId>,
}

impl DownsampleDepthPipeline {
    /// Creates a new [`DownsampleDepthPipeline`] from a bind group layout.
    ///
    /// This doesn't actually specialize the pipeline; that must be done
    /// afterward.
    fn new(bind_group_layout: BindGroupLayout) -> DownsampleDepthPipeline {
        DownsampleDepthPipeline {
            bind_group_layout,
            pipeline_id: None,
        }
    }
}

/// Stores all depth buffer downsampling pipelines.
#[derive(Resource)]
pub struct DownsampleDepthPipelines {
    /// The first pass of the pipeline, when the depth buffer is *not*
    /// multisampled.
    first: DownsampleDepthPipeline,
    /// The second pass of the pipeline, when the depth buffer is *not*
    /// multisampled.
    second: DownsampleDepthPipeline,
    /// The first pass of the pipeline, when the depth buffer is multisampled.
    first_multisample: DownsampleDepthPipeline,
    /// The second pass of the pipeline, when the depth buffer is multisampled.
    second_multisample: DownsampleDepthPipeline,
    /// The sampler that the depth downsampling shader uses to sample the depth
    /// buffer.
    sampler: Sampler,
}

/// Creates the [`DownsampleDepthPipelines`] if downsampling is supported on the
/// current platform.
fn create_downsample_depth_pipelines(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_compute_pipelines: ResMut<SpecializedComputePipelines<DownsampleDepthPipeline>>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    mut has_run: Local<bool>,
) {
    // Only run once.
    // We can't use a `resource_exists` or similar run condition here because
    // this function might fail to create downsample depth pipelines if the
    // current platform doesn't support compute shaders.
    if *has_run {
        return;
    }
    *has_run = true;

    if !gpu_preprocessing_support.is_culling_supported() {
        debug!("Downsample depth is not supported on this platform.");
        return;
    }

    // Create the bind group layouts. The bind group layouts are identical
    // between the first and second passes, so the only thing we need to
    // treat specially is the type of the first mip level (non-multisampled
    // or multisampled).
    let standard_bind_group_layout =
        create_downsample_depth_bind_group_layout(&render_device, false);
    let multisampled_bind_group_layout =
        create_downsample_depth_bind_group_layout(&render_device, true);

    // Create the depth pyramid sampler. This is shared among all shaders.
    let sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("depth pyramid sampler"),
        ..SamplerDescriptor::default()
    });

    // Initialize the pipelines.
    let mut downsample_depth_pipelines = DownsampleDepthPipelines {
        first: DownsampleDepthPipeline::new(standard_bind_group_layout.clone()),
        second: DownsampleDepthPipeline::new(standard_bind_group_layout.clone()),
        first_multisample: DownsampleDepthPipeline::new(multisampled_bind_group_layout.clone()),
        second_multisample: DownsampleDepthPipeline::new(multisampled_bind_group_layout.clone()),
        sampler,
    };

    // Specialize each pipeline with the appropriate
    // `DownsampleDepthPipelineKey`.
    downsample_depth_pipelines.first.pipeline_id = Some(specialized_compute_pipelines.specialize(
        &pipeline_cache,
        &downsample_depth_pipelines.first,
        DownsampleDepthPipelineKey::empty(),
    ));
    downsample_depth_pipelines.second.pipeline_id = Some(specialized_compute_pipelines.specialize(
        &pipeline_cache,
        &downsample_depth_pipelines.second,
        DownsampleDepthPipelineKey::SECOND_PHASE,
    ));
    downsample_depth_pipelines.first_multisample.pipeline_id =
        Some(specialized_compute_pipelines.specialize(
            &pipeline_cache,
            &downsample_depth_pipelines.first_multisample,
            DownsampleDepthPipelineKey::MULTISAMPLE,
        ));
    downsample_depth_pipelines.second_multisample.pipeline_id =
        Some(specialized_compute_pipelines.specialize(
            &pipeline_cache,
            &downsample_depth_pipelines.second_multisample,
            DownsampleDepthPipelineKey::SECOND_PHASE | DownsampleDepthPipelineKey::MULTISAMPLE,
        ));

    commands.insert_resource(downsample_depth_pipelines);
}

/// Creates a single bind group layout for the downsample depth pass.
fn create_downsample_depth_bind_group_layout(
    render_device: &RenderDevice,
    is_multisampled: bool,
) -> BindGroupLayout {
    render_device.create_bind_group_layout(
        if is_multisampled {
            "downsample multisample depth bind group layout"
        } else {
            "downsample depth bind group layout"
        },
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                // We only care about the multisample status of the depth buffer
                // for the first mip level. After the first mip level is
                // sampled, we drop to a single sample.
                if is_multisampled {
                    texture_2d_multisampled(TextureSampleType::Depth)
                } else {
                    texture_2d(TextureSampleType::Depth)
                },
                // All the mip levels follow:
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::ReadWrite),
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                sampler(SamplerBindingType::NonFiltering),
            ),
        ),
    )
}

bitflags! {
    /// Uniquely identifies a configuration of the downsample depth shader.
    ///
    /// Note that meshlets maintain their downsample depth shaders on their own
    /// and don't use this infrastructure; thus there's no flag for meshlets in
    /// here, even though the shader has defines for it.
    #[derive(Clone, Copy, PartialEq, Eq, Hash)]
    pub struct DownsampleDepthPipelineKey: u8 {
        /// True if the depth buffer is multisampled.
        const MULTISAMPLE = 1;
        /// True if this shader is the second phase of the downsample depth
        /// process; false if this shader is the first phase.
        const SECOND_PHASE = 2;
    }
}

impl SpecializedComputePipeline for DownsampleDepthPipeline {
    type Key = DownsampleDepthPipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let mut shader_defs = vec![];
        if key.contains(DownsampleDepthPipelineKey::MULTISAMPLE) {
            shader_defs.push("MULTISAMPLE".into());
        }

        let label = format!(
            "downsample depth{}{} pipeline",
            if key.contains(DownsampleDepthPipelineKey::MULTISAMPLE) {
                " multisample"
            } else {
                ""
            },
            if key.contains(DownsampleDepthPipelineKey::SECOND_PHASE) {
                " second phase"
            } else {
                " first phase"
            }
        )
        .into();

        ComputePipelineDescriptor {
            label: Some(label),
            layout: vec![self.bind_group_layout.clone()],
            push_constant_ranges: vec![PushConstantRange {
                stages: ShaderStages::COMPUTE,
                range: 0..4,
            }],
            shader: DOWNSAMPLE_DEPTH_SHADER_HANDLE,
            shader_defs,
            entry_point: if key.contains(DownsampleDepthPipelineKey::SECOND_PHASE) {
                "downsample_depth_second".into()
            } else {
                "downsample_depth_first".into()
            },
            zero_initialize_workgroup_memory: false,
        }
    }
}

/// Stores a placeholder texture that can be bound to a depth pyramid binding if
/// no depth pyramid is needed.
#[derive(Resource, Deref, DerefMut)]
pub struct DepthPyramidDummyTexture(TextureViewMeta);

impl FromWorld for DepthPyramidDummyTexture {
    fn from_world(_world: &mut World) -> Self {
        DepthPyramidDummyTexture(create_depth_pyramid_dummy_texture(
            "depth pyramid dummy texture",
            "depth pyramid dummy texture view",
        ))
    }
}

/// Creates a placeholder texture that can be bound to a depth pyramid binding
/// if no depth pyramid is needed.
pub fn create_depth_pyramid_dummy_texture(
    texture_label: &'static str,
    texture_view_label: &'static str,
) -> TextureViewMeta {
    TextureViewMeta {
        meta: ResourceMeta {
            key: String::from("depth_pyramid_dummy_texture"),
            desc: TextureInfo {
                label: Some(texture_label.into()),
                size: Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::STORAGE_BINDING,
                view_formats: vec![],
            },
        },
        texture_view_info: TextureViewInfo {
            label: Some(texture_view_label.into()),
            format: Some(TextureFormat::R32Float),
            dimension: Some(TextureViewDimension::D2),
            usage: None,
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(1),
        },
    }
}

/// Stores a hierarchical Z-buffer for a view, which is a series of mipmaps
/// useful for efficient occlusion culling.
///
/// This will only be present on a view when occlusion culling is enabled.
#[derive(Component)]
pub struct ViewDepthPyramid {
    /// A texture view containing the entire depth texture.
    pub all_mips: TextureViewMeta,
    /// A series of texture views containing one mip level each.
    pub mips: [TextureViewMeta; DEPTH_PYRAMID_MIP_COUNT],
    /// The total number of mipmap levels.
    ///
    /// This is the base-2 logarithm of the greatest dimension of the depth
    /// buffer, rounded up.
    pub mip_count: u32,
}

impl ViewDepthPyramid {
    pub fn get_view_depth_pyramid_key() -> String {
        format!("view_depth_pyramid")
    }

    /// Allocates a new depth pyramid for a depth buffer with the given size.
    pub fn new(
        depth_pyramid_dummy_texture: &TextureViewMeta,
        size: UVec2,
        texture_label: &'static str,
        texture_view_label: &'static str,
    ) -> ViewDepthPyramid {
        // Calculate the size of the depth pyramid.
        let depth_pyramid_size = Extent3d {
            width: size.x.div_ceil(2),
            height: size.y.div_ceil(2),
            depth_or_array_layers: 1,
        };

        // Calculate the number of mip levels we need.
        let depth_pyramid_mip_count = depth_pyramid_size.max_mips(TextureDimension::D2);

        let depth_pyramid = ResourceMeta {
            key: Self::get_view_depth_pyramid_key(),
            desc: TextureInfo {
                label: Some(texture_label.into()),
                size: depth_pyramid_size,
                mip_level_count: depth_pyramid_mip_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: vec![],
            },
        };

        // Create individual views for each level of the depth pyramid.
        let depth_pyramid_mips = array::from_fn(|i| {
            if (i as u32) < depth_pyramid_mip_count {
                TextureViewMeta {
                    meta: depth_pyramid.clone(),
                    texture_view_info: TextureViewInfo {
                        label: Some(texture_view_label.into()),
                        format: Some(TextureFormat::R32Float),
                        dimension: Some(TextureViewDimension::D2),
                        usage: None,
                        aspect: TextureAspect::All,
                        base_mip_level: i as u32,
                        mip_level_count: Some(1),
                        base_array_layer: 0,
                        array_layer_count: Some(1),
                    },
                }
            } else {
                (*depth_pyramid_dummy_texture).clone()
            }
        });

        // Create the view for the depth pyramid as a whole.
        let depth_pyramid_all_mips = TextureViewMeta {
            meta: depth_pyramid.clone(),
            texture_view_info: TextureViewInfo::default(),
        };

        Self {
            all_mips: depth_pyramid_all_mips,
            mips: depth_pyramid_mips,
            mip_count: depth_pyramid_mip_count,
        }
    }

    /// Creates a bind group that allows the depth buffer to be attached to the
    /// `downsample_depth.wgsl` shader.
    pub fn create_bind_group<'a>(
        &'a self,
        label: &'static str,
        bind_group_layout: &BindGroupLayout,
        source_image: BindGroupResourceHandle,
        sampler: &'a Sampler,
        frame_graph: &mut FrameGraph,
    ) -> BindGroupHandle {
        frame_graph
            .create_bind_group_handle_builder(Some(label.into()), bind_group_layout)
            .add_handle(0, source_image)
            .add_helper(1, &self.mips[0])
            .add_helper(2, &self.mips[1])
            .add_helper(3, &self.mips[2])
            .add_helper(4, &self.mips[3])
            .add_helper(5, &self.mips[4])
            .add_helper(6, &self.mips[5])
            .add_helper(7, &self.mips[6])
            .add_helper(8, &self.mips[7])
            .add_helper(9, &self.mips[8])
            .add_helper(10, &self.mips[9])
            .add_helper(11, &self.mips[10])
            .add_helper(12, &self.mips[11])
            .add_handle(13, sampler)
            .build()
    }

    /// Invokes the shaders to generate the hierarchical Z-buffer.
    ///
    /// This is intended to be invoked as part of a render node.
    pub fn downsample_depth(
        &self,
        label: &str,
        frame_graph: &mut FrameGraph,
        view_size: UVec2,
        downsample_depth_bind_group: &BindGroupHandle,
        downsample_depth_first_pipeline_id: CachedComputePipelineId,
        downsample_depth_second_pipeline_id: CachedComputePipelineId,
    ) {
        let mut pass_builder = frame_graph.create_pass_builder(label);

        let mut compute_pass_builder = pass_builder.create_compute_pass_builder();

        compute_pass_builder
            .set_pass_name(label)
            .set_compute_pipeline(downsample_depth_first_pipeline_id)
            // Pass the mip count as a push constant, for simplicity.
            .set_push_constants(0, &self.mip_count.to_le_bytes())
            .set_bind_group_handle(0, downsample_depth_bind_group, &[])
            .dispatch_workgroups(view_size.x.div_ceil(64), view_size.y.div_ceil(64), 1);

        if self.mip_count >= 7 {
            compute_pass_builder
                .set_compute_pipeline(downsample_depth_second_pipeline_id)
                .dispatch_workgroups(1, 1, 1);
        }
    }
}

/// Creates depth pyramids for views that have occlusion culling enabled.
pub fn prepare_view_depth_pyramids(
    mut commands: Commands,
    depth_pyramid_dummy_texture: Res<DepthPyramidDummyTexture>,
    views: Query<(Entity, &ExtractedView), (With<OcclusionCulling>, Without<NoIndirectDrawing>)>,
) {
    for (view_entity, view) in &views {
        commands.entity(view_entity).insert(ViewDepthPyramid::new(
            &depth_pyramid_dummy_texture,
            view.viewport.zw(),
            "view depth pyramid texture",
            "view depth pyramid texture view",
        ));
    }
}

/// The bind group that we use to attach the depth buffer and depth pyramid for
/// a view to the `downsample_depth.wgsl` shader.
///
/// This will only be present for a view if occlusion culling is enabled.
#[derive(Component, Deref, DerefMut)]
pub struct ViewDownsampleDepthBindGroup(BindGroupHandle);

/// Creates the [`ViewDownsampleDepthBindGroup`]s for all views with occlusion
/// culling enabled.
fn prepare_downsample_depth_view_bind_groups(
    mut commands: Commands,
    downsample_depth_pipelines: Res<DownsampleDepthPipelines>,
    view_depth_textures: Query<
        (
            Entity,
            &ViewDepthPyramid,
            Option<&ViewDepthTexture>,
            Option<&OcclusionCullingSubview>,
        ),
        Or<(With<ViewDepthTexture>, With<OcclusionCullingSubview>)>,
    >,
    mut frame_graph: ResMut<FrameGraph>,
) {
    for (view_entity, view_depth_pyramid, view_depth_texture, shadow_occlusion_culling) in
        &view_depth_textures
    {
        let is_multisampled = view_depth_texture
            .is_some_and(|view_depth_texture| view_depth_texture.texture.desc.sample_count > 1);

        let source_image_handle = match (view_depth_texture, shadow_occlusion_culling) {
            (Some(view_depth_texture), _) => view_depth_texture
                .texture
                .make_binding_resource_handle(&mut frame_graph),
            (None, Some(shadow_occlusion_culling)) => shadow_occlusion_culling
                .depth_texture
                .make_binding_resource_handle(&mut frame_graph),
            (None, None) => panic!("Should never happen"),
        };

        commands
            .entity(view_entity)
            .insert(ViewDownsampleDepthBindGroup(
                view_depth_pyramid.create_bind_group(
                    if is_multisampled {
                        "downsample multisample depth bind group"
                    } else {
                        "downsample depth bind group"
                    },
                    if is_multisampled {
                        &downsample_depth_pipelines
                            .first_multisample
                            .bind_group_layout
                    } else {
                        &downsample_depth_pipelines.first.bind_group_layout
                    },
                    source_image_handle,
                    &downsample_depth_pipelines.sampler,
                    &mut frame_graph,
                ),
            ));
    }
}
