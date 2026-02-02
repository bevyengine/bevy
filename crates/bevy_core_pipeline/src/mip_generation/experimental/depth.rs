//! Generation of hierarchical Z buffers for occlusion culling.
//!
//! Currently, this module only supports generation of hierarchical Z buffers
//! for occlusion culling.

use core::array;

use crate::mip_generation::DownsampleShaders;

use bevy_asset::Handle;
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    prelude::Without,
    query::{Or, With},
    resource::Resource,
    system::{Commands, Local, Query, Res, ResMut},
};
use bevy_math::{uvec2, UVec2, Vec4Swizzles as _};
use bevy_render::{
    batching::gpu_preprocessing::GpuPreprocessingSupport,
    occlusion_culling::{
        OcclusionCulling, OcclusionCullingSubview, OcclusionCullingSubviewEntities,
    },
    render_resource::{
        binding_types::{sampler, texture_2d, texture_2d_multisampled, texture_storage_2d},
        BindGroup, BindGroupEntries, BindGroupLayout, BindGroupLayoutDescriptor,
        BindGroupLayoutEntries, CachedComputePipelineId, ComputePassDescriptor, ComputePipeline,
        ComputePipelineDescriptor, Extent3d, IntoBinding, PipelineCache, Sampler,
        SamplerBindingType, SamplerDescriptor, ShaderStages, SpecializedComputePipeline,
        SpecializedComputePipelines, StorageTextureAccess, TextureAspect, TextureDescriptor,
        TextureDimension, TextureFormat, TextureSampleType, TextureUsages, TextureView,
        TextureViewDescriptor, TextureViewDimension,
    },
    renderer::{RenderContext, RenderDevice, ViewQuery},
    texture::TextureCache,
    view::{ExtractedView, NoIndirectDrawing, ViewDepthTexture},
};
use bevy_shader::Shader;
use bevy_utils::default;
use bitflags::bitflags;
use tracing::debug;

/// The maximum number of mip levels that we can produce.
///
/// 2^12 is 4096, so that's the maximum size of the depth buffer that we
/// support.
pub const DEPTH_PYRAMID_MIP_COUNT: usize = 12;

/// Produces a hierarchical Z-buffer (depth pyramid) for occlusion culling.
///
/// This runs the single-pass downsampling (SPD) shader with the *min* filter in
/// order to generate a series of mipmaps for the Z buffer. The resulting
/// hierarchical Z-buffer can be used for occlusion culling.
///
/// The *early* downsample depth pass is the first hierarchical Z-buffer stage,
/// which runs after the early prepass and before the late prepass. It prepares
/// the Z-buffer for the bounding box tests that the late mesh preprocessing
/// stage will perform.
///
/// This system won't do anything if occlusion culling isn't on.
pub fn early_downsample_depth(
    view: ViewQuery<(
        &ViewDepthPyramid,
        &ViewDownsampleDepthBindGroup,
        &ViewDepthTexture,
        Option<&OcclusionCullingSubviewEntities>,
    )>,
    shadow_view_query: Query<(
        &ViewDepthPyramid,
        &ViewDownsampleDepthBindGroup,
        &OcclusionCullingSubview,
    )>,
    downsample_depth_pipelines: Option<Res<DownsampleDepthPipelines>>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let Some(downsample_depth_pipelines) = downsample_depth_pipelines.as_deref() else {
        return;
    };

    let (
        view_depth_pyramid,
        view_downsample_depth_bind_group,
        view_depth_texture,
        maybe_view_light_entities,
    ) = view.into_inner();

    // Downsample depth for the main Z-buffer.
    downsample_depth(
        "early_downsample_depth",
        &mut ctx,
        downsample_depth_pipelines,
        &pipeline_cache,
        view_depth_pyramid,
        view_downsample_depth_bind_group,
        uvec2(
            view_depth_texture.texture.width(),
            view_depth_texture.texture.height(),
        ),
        view_depth_texture.texture.sample_count(),
    );

    // Downsample depth for shadow maps that have occlusion culling enabled.
    if let Some(view_light_entities) = maybe_view_light_entities {
        for &view_light_entity in &view_light_entities.0 {
            let Ok((view_depth_pyramid, view_downsample_depth_bind_group, occlusion_culling)) =
                shadow_view_query.get(view_light_entity)
            else {
                continue;
            };
            downsample_depth(
                "early_downsample_depth",
                &mut ctx,
                downsample_depth_pipelines,
                &pipeline_cache,
                view_depth_pyramid,
                view_downsample_depth_bind_group,
                UVec2::splat(occlusion_culling.depth_texture_size),
                1,
            );
        }
    }
}

/// Produces a hierarchical Z-buffer (depth pyramid) for occlusion culling.
///
/// This runs the single-pass downsampling (SPD) shader with the *min* filter in
/// order to generate a series of mipmaps for the Z buffer. The resulting
/// hierarchical Z-buffer can be used for occlusion culling.
///
/// The *late* downsample depth pass runs at the end of the main phase. It
/// prepares the Z-buffer for the occlusion culling that the early mesh
/// preprocessing phase of the *next* frame will perform.
///
/// This system won't do anything if occlusion culling isn't on.
pub fn late_downsample_depth(
    view: ViewQuery<(
        &ViewDepthPyramid,
        &ViewDownsampleDepthBindGroup,
        &ViewDepthTexture,
        Option<&OcclusionCullingSubviewEntities>,
    )>,
    shadow_view_query: Query<(
        &ViewDepthPyramid,
        &ViewDownsampleDepthBindGroup,
        &OcclusionCullingSubview,
    )>,
    downsample_depth_pipelines: Option<Res<DownsampleDepthPipelines>>,
    pipeline_cache: Res<PipelineCache>,
    mut ctx: RenderContext,
) {
    let Some(downsample_depth_pipelines) = downsample_depth_pipelines.as_deref() else {
        return;
    };

    let (
        view_depth_pyramid,
        view_downsample_depth_bind_group,
        view_depth_texture,
        maybe_view_light_entities,
    ) = view.into_inner();

    // Downsample depth for the main Z-buffer.
    downsample_depth(
        "late_downsample_depth",
        &mut ctx,
        downsample_depth_pipelines,
        &pipeline_cache,
        view_depth_pyramid,
        view_downsample_depth_bind_group,
        uvec2(
            view_depth_texture.texture.width(),
            view_depth_texture.texture.height(),
        ),
        view_depth_texture.texture.sample_count(),
    );

    // Downsample depth for shadow maps that have occlusion culling enabled.
    if let Some(view_light_entities) = maybe_view_light_entities {
        for &view_light_entity in &view_light_entities.0 {
            let Ok((view_depth_pyramid, view_downsample_depth_bind_group, occlusion_culling)) =
                shadow_view_query.get(view_light_entity)
            else {
                continue;
            };
            downsample_depth(
                "late_downsample_depth",
                &mut ctx,
                downsample_depth_pipelines,
                &pipeline_cache,
                view_depth_pyramid,
                view_downsample_depth_bind_group,
                UVec2::splat(occlusion_culling.depth_texture_size),
                1,
            );
        }
    }
}

/// Produces a depth pyramid from the current depth buffer for a single view.
/// The resulting depth pyramid can be used for occlusion testing.
fn downsample_depth(
    label: &str,
    ctx: &mut RenderContext,
    downsample_depth_pipelines: &DownsampleDepthPipelines,
    pipeline_cache: &PipelineCache,
    view_depth_pyramid: &ViewDepthPyramid,
    view_downsample_depth_bind_group: &ViewDownsampleDepthBindGroup,
    view_size: UVec2,
    sample_count: u32,
) {
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
        return;
    };

    // Fetch the pipelines for the two passes.
    let (Some(first_downsample_depth_pipeline), Some(second_downsample_depth_pipeline)) = (
        pipeline_cache.get_compute_pipeline(first_downsample_depth_pipeline_id),
        pipeline_cache.get_compute_pipeline(second_downsample_depth_pipeline_id),
    ) else {
        return;
    };

    // Run the depth downsampling.
    view_depth_pyramid.downsample_depth_with_ctx(
        label,
        ctx,
        view_size,
        view_downsample_depth_bind_group,
        first_downsample_depth_pipeline,
        second_downsample_depth_pipeline,
    );
}

/// A single depth downsample pipeline.
#[derive(Resource)]
pub struct DownsampleDepthPipeline {
    /// The bind group layout for this pipeline.
    pub bind_group_layout: BindGroupLayoutDescriptor,
    /// A handle that identifies the compiled shader.
    pub pipeline_id: Option<CachedComputePipelineId>,
    /// The shader asset handle.
    shader: Handle<Shader>,
}

impl DownsampleDepthPipeline {
    /// Creates a new [`DownsampleDepthPipeline`] from a bind group layout and the downsample
    /// shader.
    ///
    /// This doesn't actually specialize the pipeline; that must be done
    /// afterward.
    fn new(
        bind_group_layout: BindGroupLayoutDescriptor,
        shader: Handle<Shader>,
    ) -> DownsampleDepthPipeline {
        DownsampleDepthPipeline {
            bind_group_layout,
            pipeline_id: None,
            shader,
        }
    }
}

/// Stores all depth buffer downsampling pipelines.
#[derive(Resource)]
pub struct DownsampleDepthPipelines {
    /// The first pass of the pipeline, when the depth buffer is *not*
    /// multisampled.
    pub first: DownsampleDepthPipeline,
    /// The second pass of the pipeline, when the depth buffer is *not*
    /// multisampled.
    pub second: DownsampleDepthPipeline,
    /// The first pass of the pipeline, when the depth buffer is multisampled.
    pub first_multisample: DownsampleDepthPipeline,
    /// The second pass of the pipeline, when the depth buffer is multisampled.
    pub second_multisample: DownsampleDepthPipeline,
    /// The sampler that the depth downsampling shader uses to sample the depth
    /// buffer.
    pub sampler: Sampler,
}

/// Creates the [`DownsampleDepthPipelines`] if downsampling is supported on the
/// current platform.
pub fn create_downsample_depth_pipelines(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_compute_pipelines: ResMut<SpecializedComputePipelines<DownsampleDepthPipeline>>,
    gpu_preprocessing_support: Res<GpuPreprocessingSupport>,
    downsample_depth_shader: Res<DownsampleShaders>,
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
    let standard_bind_group_layout = create_downsample_depth_bind_group_layout(false);
    let multisampled_bind_group_layout = create_downsample_depth_bind_group_layout(true);

    // Create the depth pyramid sampler. This is shared among all shaders.
    let sampler = render_device.create_sampler(&SamplerDescriptor {
        label: Some("depth pyramid sampler"),
        ..SamplerDescriptor::default()
    });

    // Initialize the pipelines.
    let mut downsample_depth_pipelines = DownsampleDepthPipelines {
        first: DownsampleDepthPipeline::new(
            standard_bind_group_layout.clone(),
            downsample_depth_shader.depth.clone(),
        ),
        second: DownsampleDepthPipeline::new(
            standard_bind_group_layout.clone(),
            downsample_depth_shader.depth.clone(),
        ),
        first_multisample: DownsampleDepthPipeline::new(
            multisampled_bind_group_layout.clone(),
            downsample_depth_shader.depth.clone(),
        ),
        second_multisample: DownsampleDepthPipeline::new(
            multisampled_bind_group_layout.clone(),
            downsample_depth_shader.depth.clone(),
        ),
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
fn create_downsample_depth_bind_group_layout(is_multisampled: bool) -> BindGroupLayoutDescriptor {
    BindGroupLayoutDescriptor::new(
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
            immediate_size: 4,
            shader: self.shader.clone(),
            shader_defs,
            entry_point: Some(if key.contains(DownsampleDepthPipelineKey::SECOND_PHASE) {
                "downsample_depth_second".into()
            } else {
                "downsample_depth_first".into()
            }),
            ..default()
        }
    }
}

/// Stores a placeholder texture that can be bound to a depth pyramid binding if
/// no depth pyramid is needed.
#[derive(Resource, Deref, DerefMut)]
pub struct DepthPyramidDummyTexture(TextureView);

pub fn init_depth_pyramid_dummy_texture(mut commands: Commands, render_device: Res<RenderDevice>) {
    commands.insert_resource(DepthPyramidDummyTexture(
        create_depth_pyramid_dummy_texture(
            &render_device,
            "depth pyramid dummy texture",
            "depth pyramid dummy texture view",
        ),
    ));
}

/// Creates a placeholder texture that can be bound to a depth pyramid binding
/// if no depth pyramid is needed.
pub fn create_depth_pyramid_dummy_texture(
    render_device: &RenderDevice,
    texture_label: &'static str,
    texture_view_label: &'static str,
) -> TextureView {
    render_device
        .create_texture(&TextureDescriptor {
            label: Some(texture_label),
            size: Extent3d::default(),
            mip_level_count: 1,
            sample_count: 1,
            dimension: TextureDimension::D2,
            format: TextureFormat::R32Float,
            usage: TextureUsages::STORAGE_BINDING,
            view_formats: &[],
        })
        .create_view(&TextureViewDescriptor {
            label: Some(texture_view_label),
            format: Some(TextureFormat::R32Float),
            dimension: Some(TextureViewDimension::D2),
            usage: None,
            aspect: TextureAspect::All,
            base_mip_level: 0,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(1),
        })
}

/// Stores a hierarchical Z-buffer for a view, which is a series of mipmaps
/// useful for efficient occlusion culling.
///
/// This will only be present on a view when occlusion culling is enabled.
#[derive(Component)]
pub struct ViewDepthPyramid {
    /// A texture view containing the entire depth texture.
    pub all_mips: TextureView,
    /// A series of texture views containing one mip level each.
    pub mips: [TextureView; DEPTH_PYRAMID_MIP_COUNT],
    /// The total number of mipmap levels.
    ///
    /// This is the base-2 logarithm of the greatest dimension of the depth
    /// buffer, rounded up.
    pub mip_count: u32,
}

impl ViewDepthPyramid {
    /// Allocates a new depth pyramid for a depth buffer with the given size.
    pub fn new(
        render_device: &RenderDevice,
        texture_cache: &mut TextureCache,
        depth_pyramid_dummy_texture: &TextureView,
        size: UVec2,
        texture_label: &'static str,
        texture_view_label: &'static str,
    ) -> ViewDepthPyramid {
        // Calculate the size of the depth pyramid. This is the size of the
        // depth buffer rounded down to the previous power of two.
        let depth_pyramid_size = Extent3d {
            width: previous_power_of_two(size.x),
            height: previous_power_of_two(size.y),
            depth_or_array_layers: 1,
        };

        // Calculate the number of mip levels we need.
        let depth_pyramid_mip_count = depth_pyramid_size.max_mips(TextureDimension::D2);

        // Create the depth pyramid.
        let depth_pyramid = texture_cache.get(
            render_device,
            TextureDescriptor {
                label: Some(texture_label),
                size: depth_pyramid_size,
                mip_level_count: depth_pyramid_mip_count,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::R32Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        // Create individual views for each level of the depth pyramid.
        let depth_pyramid_mips = array::from_fn(|i| {
            if (i as u32) < depth_pyramid_mip_count {
                depth_pyramid.texture.create_view(&TextureViewDescriptor {
                    label: Some(texture_view_label),
                    format: Some(TextureFormat::R32Float),
                    dimension: Some(TextureViewDimension::D2),
                    usage: None,
                    aspect: TextureAspect::All,
                    base_mip_level: i as u32,
                    mip_level_count: Some(1),
                    base_array_layer: 0,
                    array_layer_count: Some(1),
                })
            } else {
                (*depth_pyramid_dummy_texture).clone()
            }
        });

        // Create the view for the depth pyramid as a whole.
        let depth_pyramid_all_mips = depth_pyramid.default_view.clone();

        Self {
            all_mips: depth_pyramid_all_mips,
            mips: depth_pyramid_mips,
            mip_count: depth_pyramid_mip_count,
        }
    }

    /// Creates a bind group that allows the depth buffer to be attached to the
    /// `downsample_depth.wgsl` shader.
    pub fn create_bind_group<'a, R>(
        &'a self,
        render_device: &RenderDevice,
        label: &'static str,
        bind_group_layout: &BindGroupLayout,
        source_image: R,
        sampler: &'a Sampler,
    ) -> BindGroup
    where
        R: IntoBinding<'a>,
    {
        render_device.create_bind_group(
            label,
            bind_group_layout,
            &BindGroupEntries::sequential((
                source_image,
                &self.mips[0],
                &self.mips[1],
                &self.mips[2],
                &self.mips[3],
                &self.mips[4],
                &self.mips[5],
                &self.mips[6],
                &self.mips[7],
                &self.mips[8],
                &self.mips[9],
                &self.mips[10],
                &self.mips[11],
                sampler,
            )),
        )
    }

    pub fn downsample_depth_with_ctx(
        &self,
        label: &str,
        ctx: &mut RenderContext,
        view_size: UVec2,
        downsample_depth_bind_group: &BindGroup,
        downsample_depth_first_pipeline: &ComputePipeline,
        downsample_depth_second_pipeline: &ComputePipeline,
    ) {
        // We need to make sure that every mip level the single-pass
        // downsampling (SPD) shader sees has lengths that are powers of two for
        // correct conservative depth buffer downsampling. To do this, we
        // maintain the fiction that we're downsampling a depth buffer scaled up
        // so that it has side lengths rounded up to the next power of two. (If
        // the depth buffer already has a side length that's a power of two,
        // then we double it anyway; this ensures that we don't lose any
        // precision in the top level of the depth pyramid.) The
        // `downsample_depth` shader's `load_mip_0` function returns the value
        // that sampling such a depth buffer would yield, without actually
        // having to construct such a scaled depth buffer.
        let virtual_view_size = uvec2(
            (view_size.x + 1).next_power_of_two(),
            (view_size.y + 1).next_power_of_two(),
        );

        let command_encoder = ctx.command_encoder();
        let mut downsample_pass = command_encoder.begin_compute_pass(&ComputePassDescriptor {
            label: Some(label),
            timestamp_writes: None,
        });
        downsample_pass.set_pipeline(downsample_depth_first_pipeline);
        // Pass the mip count as an immediate, for simplicity.
        downsample_pass.set_immediates(0, &self.mip_count.to_le_bytes());
        downsample_pass.set_bind_group(0, downsample_depth_bind_group, &[]);
        downsample_pass.dispatch_workgroups(
            virtual_view_size.x.div_ceil(64),
            virtual_view_size.y.div_ceil(64),
            1,
        );

        if self.mip_count >= 7 {
            downsample_pass.set_pipeline(downsample_depth_second_pipeline);
            downsample_pass.dispatch_workgroups(1, 1, 1);
        }
    }
}

/// Creates depth pyramids for views that have occlusion culling enabled.
pub fn prepare_view_depth_pyramids(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    depth_pyramid_dummy_texture: Res<DepthPyramidDummyTexture>,
    views: Query<(Entity, &ExtractedView), (With<OcclusionCulling>, Without<NoIndirectDrawing>)>,
) {
    for (view_entity, view) in &views {
        commands.entity(view_entity).insert(ViewDepthPyramid::new(
            &render_device,
            &mut texture_cache,
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
pub struct ViewDownsampleDepthBindGroup(BindGroup);

/// Creates the [`ViewDownsampleDepthBindGroup`]s for all views with occlusion
/// culling enabled.
pub fn prepare_downsample_depth_view_bind_groups(
    mut commands: Commands,
    render_device: Res<RenderDevice>,
    downsample_depth_pipelines: Res<DownsampleDepthPipelines>,
    pipeline_cache: Res<PipelineCache>,
    view_depth_textures: Query<
        (
            Entity,
            &ViewDepthPyramid,
            Option<&ViewDepthTexture>,
            Option<&OcclusionCullingSubview>,
        ),
        Or<(With<ViewDepthTexture>, With<OcclusionCullingSubview>)>,
    >,
) {
    for (view_entity, view_depth_pyramid, view_depth_texture, shadow_occlusion_culling) in
        &view_depth_textures
    {
        let is_multisampled = view_depth_texture
            .is_some_and(|view_depth_texture| view_depth_texture.texture.sample_count() > 1);
        commands
            .entity(view_entity)
            .insert(ViewDownsampleDepthBindGroup(
                view_depth_pyramid.create_bind_group(
                    &render_device,
                    if is_multisampled {
                        "downsample multisample depth bind group"
                    } else {
                        "downsample depth bind group"
                    },
                    &pipeline_cache.get_bind_group_layout(if is_multisampled {
                        &downsample_depth_pipelines
                            .first_multisample
                            .bind_group_layout
                    } else {
                        &downsample_depth_pipelines.first.bind_group_layout
                    }),
                    match (view_depth_texture, shadow_occlusion_culling) {
                        (Some(view_depth_texture), _) => view_depth_texture.view(),
                        (None, Some(shadow_occlusion_culling)) => {
                            &shadow_occlusion_culling.depth_texture_view
                        }
                        (None, None) => panic!("Should never happen"),
                    },
                    &downsample_depth_pipelines.sampler,
                ),
            ));
    }
}

/// Returns the previous power of two of x, or, if x is exactly a power of two,
/// returns x unchanged.
fn previous_power_of_two(x: u32) -> u32 {
    1 << (31 - x.leading_zeros())
}
