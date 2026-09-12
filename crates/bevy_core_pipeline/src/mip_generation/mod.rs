//! Downsampling of textures to produce mipmap levels.
//!
//! This module implements variations on the [AMD FidelityFX single-pass
//! downsampling] shader. It's used for generating mipmaps for textures
//! ([`MipGenerationJobs`]) and for creating hierarchical Z-buffers (the
//! [`experimental::depth`] module).
//!
//! See the documentation for [`MipGenerationJobs`] and [`experimental::depth`]
//! for more information.
//!
//! [AMD FidelityFX single-pass downsampling]: https://gpuopen.com/fidelityfx-spd/

use crate::core_3d::prepare_core_3d_depth_textures;
use crate::deferred::node::early_deferred_prepass;
use crate::mip_generation::experimental::depth::{
    self, early_downsample_depth, late_downsample_depth, DownsampleDepthPipeline,
    DownsampleDepthPipelines,
};
use crate::prepass::node::late_prepass;
use crate::schedule::{Core3d, Core3dSystems};

use bevy_app::{App, Plugin};
use bevy_asset::{embedded_asset, load_embedded_asset, AssetId, Handle};
use bevy_derive::{Deref, DerefMut};
use bevy_ecs::{
    prelude::resource_exists,
    resource::Resource,
    schedule::IntoScheduleConfigs as _,
    system::{Res, ResMut},
    world::{FromWorld, World},
};
use bevy_image::Image;
use bevy_log::error;
use bevy_math::{vec2, Vec2};
use bevy_platform::collections::{hash_map::Entry, HashMap, HashSet};
use bevy_render::{
    diagnostic::RecordDiagnostics as _,
    render_asset::RenderAssets,
    render_resource::{
        binding_types::{
            sampler, texture_2d, texture_2d_array, texture_storage_2d, texture_storage_2d_array,
            uniform_buffer,
        },
        BindGroup, BindGroupEntries, BindGroupLayoutDescriptor, BindGroupLayoutEntries,
        CachedComputePipelineId, ComputePassDescriptor, ComputePipelineDescriptor, Extent3d,
        FilterMode, MipmapFilterMode, PipelineCache, Sampler, SamplerBindingType,
        SamplerDescriptor, ShaderStages, ShaderType, SpecializedComputePipeline,
        SpecializedComputePipelines, StorageTextureAccess, TextureAspect, TextureDescriptor,
        TextureDimension, TextureFormat, TextureFormatFeatureFlags, TextureUsages, TextureView,
        TextureViewDescriptor, TextureViewDimension, UniformBuffer,
    },
    renderer::{RenderAdapter, RenderContext, RenderDevice, RenderQueue},
    settings::WgpuFeatures,
    texture::GpuImage,
    RenderStartup,
};
use bevy_render::{GpuResourceAppExt, Render, RenderApp, RenderSystems};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;

pub mod experimental;

/// A resource that stores the shaders that perform downsampling.
#[derive(Clone, Resource)]
pub struct DownsampleShaders {
    /// The experimental shader that downsamples depth
    /// (`downsample_depth.wesl`).
    pub depth: Handle<Shader>,
    /// The shader that performs downsampling of color textures
    /// (`downsample.wesl`).
    ///
    /// [`DownsamplePipeline`] specializes it for a texture format.
    pub general: Handle<Shader>,
}

/// The single-pass downsampling compute pipeline for color textures.
///
/// Specialize it with [`SpecializedComputePipelines`] and a
/// [`DownsamplePipelineKey`], once per [`DownsamplePass`].
#[derive(Resource)]
pub struct DownsamplePipeline {
    /// The `downsample.wesl` shader.
    shader: Handle<Shader>,
    /// Whether the device supports subgroup operations.
    subgroup_support: bool,
}

impl DownsamplePipeline {
    /// Returns true if the shader can downsample textures in `format`.
    ///
    /// Only float formats are supported because the shader stores `vec4f`.
    pub fn supports_texture_format(format: TextureFormat) -> bool {
        texture_format_shader_def(format).is_some()
    }
}

impl FromWorld for DownsamplePipeline {
    fn from_world(world: &mut World) -> Self {
        DownsamplePipeline {
            shader: world.resource::<DownsampleShaders>().general.clone(),
            subgroup_support: world
                .resource::<RenderDevice>()
                .features()
                .contains(WgpuFeatures::SUBGROUP),
        }
    }
}

/// Selects a variant of the [`DownsamplePipeline`].
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct DownsamplePipelineKey {
    /// The format of the texture to downsample.
    ///
    /// Must be a format that [`DownsamplePipeline::supports_texture_format`]
    /// accepts.
    pub texture_format: TextureFormat,
    /// True if the texture is a 2D array texture, such as a cubemap.
    pub array_texture: bool,
    /// True if both passes share a single bind group.
    ///
    /// See [`can_combine_downsampling_bind_groups`].
    pub combine_bind_groups: bool,
    /// The pass that this pipeline runs.
    pub pass: DownsamplePass,
}

/// One of the two dispatches of the single-pass downsampling shader.
///
/// Note that, despite the name, the single-pass downsampling shader has two
/// passes, not one. This is because WGSL doesn't presently support
/// globally-coherent buffers; the only way to have a synchronization point is
/// to issue a second dispatch.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum DownsamplePass {
    /// Produces mip levels 1 to 6.
    First,
    /// Produces mip levels 7 to 12.
    Second,
}

impl SpecializedComputePipeline for DownsamplePipeline {
    type Key = DownsamplePipelineKey;

    fn specialize(&self, key: Self::Key) -> ComputePipelineDescriptor {
        let mut shader_defs = vec![
            texture_format_shader_def(key.texture_format).unwrap_or_else(|| {
                panic!(
                    "The downsample shader doesn't support texture format {:?}",
                    key.texture_format
                )
            }),
            ShaderDefVal::Bool("ARRAY_TEXTURE".into(), key.array_texture),
        ];
        if self.subgroup_support {
            shader_defs.push("SUBGROUP_SUPPORT".into());
        }
        shader_defs.push(
            match (key.combine_bind_groups, key.pass) {
                (true, _) => "COMBINE_BIND_GROUP",
                (false, DownsamplePass::First) => "SPLIT_BIND_GROUP_FIRST",
                (false, DownsamplePass::Second) => "SPLIT_BIND_GROUP_SECOND",
            }
            .into(),
        );

        let (pass_number, entry_point) = match key.pass {
            DownsamplePass::First => (1, "downsample_first"),
            DownsamplePass::Second => (2, "downsample_second"),
        };

        ComputePipelineDescriptor {
            label: Some(
                format!(
                    "mip generation pipeline, pass {pass_number} ({:?})",
                    key.texture_format
                )
                .into(),
            ),
            layout: vec![key.bind_group_layout()],
            shader: self.shader.clone(),
            shader_defs,
            entry_point: Some(entry_point.into()),
            ..default()
        }
    }
}

// The number of storage textures required to combine the bind groups in the
// downsampling shader.
const REQUIRED_STORAGE_TEXTURES: u32 = 12;

/// Returns the shader def that selects the output format of `downsample.wesl`,
/// or `None` if the format isn't supported. Only float formats are supported
/// because the shader stores `vec4f`.
fn texture_format_shader_def(format: TextureFormat) -> Option<ShaderDefVal> {
    let name = match format {
        TextureFormat::Rgba8Unorm => "TEXTURE_FORMAT_RGBA8UNORM",
        TextureFormat::Rgba8Snorm => "TEXTURE_FORMAT_RGBA8SNORM",
        TextureFormat::Rgba16Unorm => "TEXTURE_FORMAT_RGBA16UNORM",
        TextureFormat::Rgba16Snorm => "TEXTURE_FORMAT_RGBA16SNORM",
        TextureFormat::Rgba16Float => "TEXTURE_FORMAT_RGBA16FLOAT",
        TextureFormat::Rg8Unorm => "TEXTURE_FORMAT_RG8UNORM",
        TextureFormat::Rg8Snorm => "TEXTURE_FORMAT_RG8SNORM",
        TextureFormat::Rg16Unorm => "TEXTURE_FORMAT_RG16UNORM",
        TextureFormat::Rg16Snorm => "TEXTURE_FORMAT_RG16SNORM",
        TextureFormat::Rg16Float => "TEXTURE_FORMAT_RG16FLOAT",
        TextureFormat::R32Float => "TEXTURE_FORMAT_R32FLOAT",
        TextureFormat::Rg32Float => "TEXTURE_FORMAT_RG32FLOAT",
        TextureFormat::Rgba32Float => "TEXTURE_FORMAT_RGBA32FLOAT",
        TextureFormat::Bgra8Unorm => "TEXTURE_FORMAT_BGRA8UNORM",
        TextureFormat::R8Unorm => "TEXTURE_FORMAT_R8UNORM",
        TextureFormat::R8Snorm => "TEXTURE_FORMAT_R8SNORM",
        TextureFormat::R16Unorm => "TEXTURE_FORMAT_R16UNORM",
        TextureFormat::R16Snorm => "TEXTURE_FORMAT_R16SNORM",
        TextureFormat::R16Float => "TEXTURE_FORMAT_R16FLOAT",
        TextureFormat::Rgb10a2Unorm => "TEXTURE_FORMAT_RGB10A2UNORM",
        _ => return None,
    };
    Some(name.into())
}

/// A render-world resource that stores a list of [`Image`]s that will have
/// mipmaps generated for them.
///
/// You can add images to this list via the [`MipGenerationJobs::add`] method,
/// in the render world. Note that this, by itself, isn't enough to generate
/// the mipmaps; you must also add a [`generate_mips_for_phase`] system to the render schedule.
///
/// This resource exists only in the render world, not the main world.
/// Therefore, you typically want to place images in this resource in a system
/// that runs in the [`bevy_render::ExtractSchedule`] of the
/// [`bevy_render::RenderApp`].
///
/// See `dynamic_mip_generation` for an example of usage.
#[derive(Resource, Default, Deref, DerefMut)]
pub struct MipGenerationJobs(pub HashMap<MipGenerationPhaseId, MipGenerationPhase>);

impl MipGenerationJobs {
    /// Schedules the generation of mipmaps for an image.
    ///
    /// Mipmaps will be generated during the execution of the
    /// [`generate_mips_for_phase`] system corresponding to the [`MipGenerationPhaseId`].
    /// Note that, by default, Bevy doesn't automatically add any such system to
    /// the render schedule; it's up to you to manually add that system.
    pub fn add(&mut self, phase: MipGenerationPhaseId, image: impl Into<AssetId<Image>>) {
        self.entry(phase).or_default().push(image.into());
    }
}

/// The list of [`Image`]s that will have mipmaps generated for them during a
/// specific phase.
///
/// The [`MipGenerationJobs`] resource stores one of these lists per mipmap
/// generation phase.
///
/// To add images to this list, use [`MipGenerationJobs::add`] in a render app
/// system.
#[derive(Default, Deref, DerefMut)]
pub struct MipGenerationPhase(pub Vec<AssetId<Image>>);

/// Identifies a *phase* during which mipmaps will be generated for an image.
///
/// Sometimes, mipmaps must be generated at a specific time during the rendering
/// process. This typically occurs when a camera renders to the image and then
/// the image is sampled later in the frame as a second camera renders the
/// scene. In this case, the mipmaps must be generated after the first camera
/// renders to the image rendered to but before the second camera's rendering
/// samples the image. To express these kinds of dependencies, you group images
/// into *phases* and schedule systems that call [`generate_mips_for_phase`]
/// targeting each phase at the appropriate time.
///
/// Each phase has an ID, which is an arbitrary 32-bit integer. You may specify
/// any value you wish as a phase ID, so long as the system that calls
/// [`generate_mips_for_phase`] uses the same ID.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct MipGenerationPhaseId(pub u32);

/// Stores the pipelines and bind groups for each image that has mipmaps
/// generated for it.
///
/// The `prepare_mip_generator_pipelines` system adds an entry when an image is
/// first scheduled and removes it on the first frame the image isn't.
#[derive(Resource, Default)]
pub struct MipGenerationPipelines {
    jobs: HashMap<AssetId<Image>, MipGenerationJobResources>,
}

/// Pipelines and bind groups for the downsampling shader associated with a
/// single texture.
struct MipGenerationJobResources {
    downsampling_pipeline_pass_1: CachedComputePipelineId,
    downsampling_pipeline_pass_2: CachedComputePipelineId,
    downsampling_bind_group_pass_1: BindGroup,
    downsampling_bind_group_pass_2: BindGroup,
}

/// Constants for the single-pass downsampling shader generated on the CPU and
/// read on the GPU.
///
/// These constants are stored within a uniform buffer. There's one such uniform
/// buffer per image.
#[derive(Clone, Copy, ShaderType)]
#[repr(C)]
pub struct DownsamplingConstants {
    /// The number of mip levels that this image possesses.
    pub mips: u32,
    /// The reciprocal of the size of the first mipmap level for this texture.
    pub inverse_input_size: Vec2,
    /// Padding.
    pub _padding: u32,
}

/// A plugin that allows Bevy to repeatedly downsample textures to create
/// mipmaps.
///
/// Generation of mipmaps happens on the GPU.
pub struct MipGenerationPlugin;

impl Plugin for MipGenerationPlugin {
    fn build(&self, app: &mut App) {
        embedded_asset!(app, "experimental/downsample_depth.wesl");
        embedded_asset!(app, "downsample.wesl");

        let depth_shader = load_embedded_asset!(app, "experimental/downsample_depth.wesl");

        let downsample_shaders = DownsampleShaders {
            depth: depth_shader,
            general: load_embedded_asset!(app, "downsample.wesl"),
        };
        app.insert_resource(downsample_shaders.clone());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_gpu_resource::<SpecializedComputePipelines<DownsampleDepthPipeline>>()
            .init_gpu_resource::<DownsamplePipeline>()
            .init_gpu_resource::<SpecializedComputePipelines<DownsamplePipeline>>()
            .init_resource::<MipGenerationJobs>()
            .init_gpu_resource::<MipGenerationPipelines>()
            .init_gpu_resource::<MipGenerationResources>()
            .insert_resource(downsample_shaders)
            .add_systems(RenderStartup, depth::init_depth_pyramid_dummy_texture)
            .add_systems(
                Core3d,
                (
                    early_downsample_depth
                        .after(early_deferred_prepass)
                        .before(late_prepass),
                    late_downsample_depth.in_set(Core3dSystems::PostProcess),
                ),
            )
            .add_systems(
                Render,
                depth::create_downsample_depth_pipelines.in_set(RenderSystems::Prepare),
            )
            .add_systems(
                Render,
                (
                    depth::prepare_view_depth_pyramids,
                    depth::prepare_downsample_depth_view_bind_groups,
                )
                    .chain()
                    .in_set(RenderSystems::PrepareResources)
                    .run_if(resource_exists::<DownsampleDepthPipelines>)
                    .after(prepare_core_3d_depth_textures),
            )
            .add_systems(
                Render,
                prepare_mip_generator_pipelines.in_set(RenderSystems::PrepareResources),
            )
            .add_systems(
                Render,
                reset_mip_generation_jobs.in_set(RenderSystems::Cleanup),
            );
    }
}

/// Global GPU resources that the mip generation pipelines use.
///
/// At the moment, the only such resource is a texture sampler.
#[derive(Resource)]
struct MipGenerationResources {
    /// The texture sampler that the single-pass downsampling pipelines use to
    /// sample the source texture.
    sampler: Sampler,
}

impl FromWorld for MipGenerationResources {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource_mut::<RenderDevice>();
        MipGenerationResources {
            sampler: render_device.create_sampler(&SamplerDescriptor {
                label: Some("mip generation sampler"),
                mag_filter: FilterMode::Linear,
                min_filter: FilterMode::Linear,
                mipmap_filter: MipmapFilterMode::Nearest,
                ..default()
            }),
        }
    }
}

/// Generates mipmaps for all images in a [`MipGenerationPhaseId`].
///
/// This function should be called from within a render system to generate
/// mipmaps for all images that have been enqueued for the specified phase.
/// The phased nature of mipmap generation allows precise control over the time
/// when mipmaps are generated for each image. Your system should be ordered
/// so that the mipmaps will be generated after any passes that *write* to the
/// images in question but before any shaders that *read* from those images
/// execute.
///
/// See `dynamic_mip_generation` for an example of use.
pub fn generate_mips_for_phase(
    phase_id: MipGenerationPhaseId,
    mip_generation_jobs: &MipGenerationJobs,
    pipeline_cache: &PipelineCache,
    mip_generation_pipelines: &MipGenerationPipelines,
    gpu_images: &RenderAssets<GpuImage>,
    ctx: &mut RenderContext,
) {
    let Some(mip_generation_phase) = mip_generation_jobs.get(&phase_id) else {
        return;
    };
    if mip_generation_phase.is_empty() {
        // Quickly bail out if there's nothing to do.
        return;
    }

    let diagnostics = ctx.diagnostic_recorder();
    let diagnostics = diagnostics.as_deref();

    for mip_generation_job in mip_generation_phase.iter() {
        let Some(gpu_image) = gpu_images.get(*mip_generation_job) else {
            continue;
        };
        let Some(job_resources) = mip_generation_pipelines.jobs.get(mip_generation_job) else {
            continue;
        };

        // Fetch the mip generation pipelines.
        let (Some(mip_generation_pipeline_pass_1), Some(mip_generation_pipeline_pass_2)) = (
            pipeline_cache.get_compute_pipeline(job_resources.downsampling_pipeline_pass_1),
            pipeline_cache.get_compute_pipeline(job_resources.downsampling_pipeline_pass_2),
        ) else {
            continue;
        };

        // Perform the first downsampling pass.
        {
            let mut compute_pass_1 =
                ctx.command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("mip generation pass 1"),
                        timestamp_writes: None,
                    });
            let pass_span = diagnostics.pass_span(&mut compute_pass_1, "mip generation pass 1");
            compute_pass_1.set_pipeline(mip_generation_pipeline_pass_1);
            compute_pass_1.set_bind_group(0, &job_resources.downsampling_bind_group_pass_1, &[]);
            compute_pass_1.dispatch_workgroups(
                gpu_image.texture_descriptor.size.width.div_ceil(64),
                gpu_image.texture_descriptor.size.height.div_ceil(64),
                1,
            );
            pass_span.end(&mut compute_pass_1);
        }

        // Perform the second downsampling pass.
        {
            let mut compute_pass_2 =
                ctx.command_encoder()
                    .begin_compute_pass(&ComputePassDescriptor {
                        label: Some("mip generation pass 2"),
                        timestamp_writes: None,
                    });
            let pass_span = diagnostics.pass_span(&mut compute_pass_2, "mip generation pass 2");
            compute_pass_2.set_pipeline(mip_generation_pipeline_pass_2);
            compute_pass_2.set_bind_group(0, &job_resources.downsampling_bind_group_pass_2, &[]);
            compute_pass_2.dispatch_workgroups(
                gpu_image.texture_descriptor.size.width.div_ceil(256),
                gpu_image.texture_descriptor.size.height.div_ceil(256),
                1,
            );
            pass_span.end(&mut compute_pass_2);
        }
    }
}

/// Creates all bind group layouts, bind groups, and pipelines for all mipmap
/// generation jobs that have been enqueued this frame.
///
/// Bind group layouts, bind groups, and pipelines are all cached for images
/// that are being processed every frame.
fn prepare_mip_generator_pipelines(
    mip_generation_pipelines: ResMut<MipGenerationPipelines>,
    mip_generation_resources: Res<MipGenerationResources>,
    mip_generation_jobs: Res<MipGenerationJobs>,
    pipeline_cache: Res<PipelineCache>,
    mut specialized_pipelines: ResMut<SpecializedComputePipelines<DownsamplePipeline>>,
    downsample_pipeline: Res<DownsamplePipeline>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    render_adapter: Res<RenderAdapter>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let mip_generation_pipelines = mip_generation_pipelines.into_inner();

    // Check to see whether we can combine downsampling bind groups on this
    // hardware and driver.
    let combine_downsampling_bind_groups =
        can_combine_downsampling_bind_groups(&render_adapter, &render_device);

    // Make a record of all jobs that we saw so that we can expire cached bind
    // groups at the end of this process.
    let mut all_source_images = HashSet::new();

    for mip_generation_phase in mip_generation_jobs.values() {
        for mip_generation_job in mip_generation_phase.iter() {
            let Some(gpu_image) = gpu_images.get(*mip_generation_job) else {
                continue;
            };

            // Note this job.
            all_source_images.insert(mip_generation_job);

            // Create pipelines and bind groups for the job if it's new.
            let Entry::Vacant(vacant_entry) =
                mip_generation_pipelines.jobs.entry(*mip_generation_job)
            else {
                continue;
            };

            let target_format = gpu_image.texture_descriptor.format;
            if !DownsamplePipeline::supports_texture_format(target_format) {
                error!(
                    "Attempted to generate mips for texture {:?} with format {:?}, but no \
                     downsample shader was available for that texture format",
                    mip_generation_job, target_format
                );
                continue;
            }

            let key_pass_1 = DownsamplePipelineKey {
                texture_format: target_format,
                array_texture: false,
                combine_bind_groups: combine_downsampling_bind_groups,
                pass: DownsamplePass::First,
            };
            let key_pass_2 = DownsamplePipelineKey {
                pass: DownsamplePass::Second,
                ..key_pass_1
            };

            let downsampling_constants_buffer =
                create_downsampling_constants_buffer(&render_device, &render_queue, gpu_image);

            let (downsampling_bind_group_pass_1, downsampling_bind_group_pass_2) =
                create_downsampling_bind_groups(
                    &render_device,
                    &pipeline_cache,
                    &mip_generation_resources,
                    &downsampling_constants_buffer,
                    gpu_image,
                    key_pass_1,
                    key_pass_2,
                );

            vacant_entry.insert(MipGenerationJobResources {
                downsampling_pipeline_pass_1: specialized_pipelines.specialize(
                    &pipeline_cache,
                    &downsample_pipeline,
                    key_pass_1,
                ),
                downsampling_pipeline_pass_2: specialized_pipelines.specialize(
                    &pipeline_cache,
                    &downsample_pipeline,
                    key_pass_2,
                ),
                downsampling_bind_group_pass_1,
                downsampling_bind_group_pass_2,
            });
        }
    }

    // Expire all bind groups for jobs that we didn't see this frame.
    //
    // Note that this logic ensures that we don't recreate bind groups for
    // images that are updated every frame.
    mip_generation_pipelines
        .jobs
        .retain(|asset_id, _| all_source_images.contains(asset_id));
}

impl DownsamplePipelineKey {
    /// Returns the [`BindGroupLayoutDescriptor`] for the pipeline that this
    /// key selects.
    pub fn bind_group_layout(&self) -> BindGroupLayoutDescriptor {
        let texture_sample_type = self.texture_format.sample_type(None, None).expect(
        "Depth and multisample texture formats shouldn't have mip generation shaders to begin with",
    );
        let source_texture = if self.array_texture {
            texture_2d_array(texture_sample_type)
        } else {
            texture_2d(texture_sample_type)
        };
        let storage = |access| {
            if self.array_texture {
                texture_storage_2d_array(self.texture_format, access)
            } else {
                texture_storage_2d(self.texture_format, access)
            }
        };
        let mips_storage = storage(StorageTextureAccess::WriteOnly);

        if self.combine_bind_groups {
            let mip6_storage = storage(StorageTextureAccess::ReadWrite);
            return BindGroupLayoutDescriptor::new(
                "combined mip generation bind group layout",
                &BindGroupLayoutEntries::sequential(
                    ShaderStages::COMPUTE,
                    (
                        sampler(SamplerBindingType::Filtering),
                        uniform_buffer::<DownsamplingConstants>(false),
                        source_texture,
                        mips_storage, // 1
                        mips_storage, // 2
                        mips_storage, // 3
                        mips_storage, // 4
                        mips_storage, // 5
                        mip6_storage, // 6
                        mips_storage, // 7
                        mips_storage, // 8
                        mips_storage, // 9
                        mips_storage, // 10
                        mips_storage, // 11
                        mips_storage, // 12
                    ),
                ),
            );
        }

        let label = match self.pass {
            DownsamplePass::First => "mip generation bind group layout, pass 1",
            DownsamplePass::Second => "mip generation bind group layout, pass 2",
        };
        BindGroupLayoutDescriptor::new(
            label,
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<DownsamplingConstants>(false),
                    source_texture, // input mip
                    mips_storage,   // output mip 1
                    mips_storage,   // output mip 2
                    mips_storage,   // output mip 3
                    mips_storage,   // output mip 4
                    mips_storage,   // output mip 5
                    mips_storage,   // output mip 6
                ),
            ),
        )
    }
}

/// Creates the bind groups for the single-pass downsampling shader associated
/// with a single texture.
///
/// Depending on whether bind groups can be combined on this platform, this
/// returns either two copies of a single bind group or two separate bind
/// groups.
fn create_downsampling_bind_groups(
    render_device: &RenderDevice,
    pipeline_cache: &PipelineCache,
    mip_generation_resources: &MipGenerationResources,
    downsampling_constants_buffer: &UniformBuffer<DownsamplingConstants>,
    gpu_image: &GpuImage,
    key_pass_1: DownsamplePipelineKey,
    key_pass_2: DownsamplePipelineKey,
) -> (BindGroup, BindGroup) {
    let input_texture_view_pass_1 = gpu_image.texture.create_view(&TextureViewDescriptor {
        label: Some("mip generation input texture view, pass 1"),
        format: Some(gpu_image.texture.format()),
        dimension: Some(TextureViewDimension::D2),
        base_mip_level: 0,
        mip_level_count: Some(1),
        ..default()
    });

    // If we can combine downsampling bind groups on this platform, we only need
    // one bind group.
    if key_pass_1.combine_bind_groups {
        let bind_group = render_device.create_bind_group(
            Some("combined mip generation bind group"),
            &pipeline_cache.get_bind_group_layout(&key_pass_1.bind_group_layout()),
            &BindGroupEntries::sequential((
                &mip_generation_resources.sampler,
                downsampling_constants_buffer,
                &input_texture_view_pass_1,
                &get_mip_storage_view(render_device, gpu_image, 1),
                &get_mip_storage_view(render_device, gpu_image, 2),
                &get_mip_storage_view(render_device, gpu_image, 3),
                &get_mip_storage_view(render_device, gpu_image, 4),
                &get_mip_storage_view(render_device, gpu_image, 5),
                &get_mip_storage_view(render_device, gpu_image, 6),
                &get_mip_storage_view(render_device, gpu_image, 7),
                &get_mip_storage_view(render_device, gpu_image, 8),
                &get_mip_storage_view(render_device, gpu_image, 9),
                &get_mip_storage_view(render_device, gpu_image, 10),
                &get_mip_storage_view(render_device, gpu_image, 11),
                &get_mip_storage_view(render_device, gpu_image, 12),
            )),
        );
        return (bind_group.clone(), bind_group);
    }

    // Otherwise, create two separate bind groups.

    let input_texture_view_pass_2 = gpu_image.texture.create_view(&TextureViewDescriptor {
        label: Some("mip generation input texture view, pass 2"),
        format: Some(gpu_image.texture.format()),
        dimension: Some(TextureViewDimension::D2),
        base_mip_level: gpu_image.texture_descriptor.mip_level_count.min(6),
        mip_level_count: Some(1),
        ..default()
    });

    let bind_group_pass_1 = render_device.create_bind_group(
        "mip generation bind group, pass 1",
        &pipeline_cache.get_bind_group_layout(&key_pass_1.bind_group_layout()),
        &BindGroupEntries::sequential((
            &mip_generation_resources.sampler,
            downsampling_constants_buffer,
            &input_texture_view_pass_1,
            &get_mip_storage_view(render_device, gpu_image, 1),
            &get_mip_storage_view(render_device, gpu_image, 2),
            &get_mip_storage_view(render_device, gpu_image, 3),
            &get_mip_storage_view(render_device, gpu_image, 4),
            &get_mip_storage_view(render_device, gpu_image, 5),
            &get_mip_storage_view(render_device, gpu_image, 6),
        )),
    );
    let bind_group_pass_2 = render_device.create_bind_group(
        "mip generation bind group, pass 2",
        &pipeline_cache.get_bind_group_layout(&key_pass_2.bind_group_layout()),
        &BindGroupEntries::sequential((
            &mip_generation_resources.sampler,
            downsampling_constants_buffer,
            &input_texture_view_pass_2,
            &get_mip_storage_view(render_device, gpu_image, 7),
            &get_mip_storage_view(render_device, gpu_image, 8),
            &get_mip_storage_view(render_device, gpu_image, 9),
            &get_mip_storage_view(render_device, gpu_image, 10),
            &get_mip_storage_view(render_device, gpu_image, 11),
            &get_mip_storage_view(render_device, gpu_image, 12),
        )),
    );

    (bind_group_pass_1, bind_group_pass_2)
}

/// Creates the uniform buffer containing the [`DownsamplingConstants`] for a
/// single texture.
fn create_downsampling_constants_buffer(
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
    gpu_image: &GpuImage,
) -> UniformBuffer<DownsamplingConstants> {
    let downsampling_constants = DownsamplingConstants {
        mips: gpu_image.texture_descriptor.mip_level_count,
        inverse_input_size: vec2(
            1.0 / gpu_image.texture_descriptor.size.width as f32,
            1.0 / gpu_image.texture_descriptor.size.height as f32,
        ),
        _padding: 0,
    };

    let mut downsampling_constants_buffer = UniformBuffer::from(downsampling_constants);
    downsampling_constants_buffer.write_buffer(render_device, render_queue);
    downsampling_constants_buffer
}

/// Returns a view of the given mipmap level of a texture, suitable for
/// attachment as a texture storage binding.
fn get_mip_storage_view(
    render_device: &RenderDevice,
    gpu_image: &GpuImage,
    level: u32,
) -> TextureView {
    // If `level` represents an actual mip level of the image, return a view to
    // it.
    if level < gpu_image.texture_descriptor.mip_level_count {
        return gpu_image.texture.create_view(&TextureViewDescriptor {
            label: Some(&*format!(
                "mip downsampling storage view {}/{}",
                level, gpu_image.texture_descriptor.mip_level_count
            )),
            format: Some(gpu_image.texture_descriptor.format),
            dimension: Some(TextureViewDimension::D2),
            aspect: TextureAspect::All,
            base_mip_level: level,
            mip_level_count: Some(1),
            base_array_layer: 0,
            array_layer_count: Some(1),
            usage: Some(TextureUsages::STORAGE_BINDING),
        });
    }

    // Otherwise, create a dummy texture and return a view to that.

    let dummy_texture = render_device.create_texture(&TextureDescriptor {
        label: Some(&*format!(
            "mip downsampling dummy storage view {}/{}",
            level, gpu_image.texture_descriptor.mip_level_count
        )),
        size: Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: gpu_image.texture_descriptor.format,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    dummy_texture.create_view(&TextureViewDescriptor::default())
}

/// A system that clears out the [`MipGenerationJobs`] resource in preparation
/// for a new frame.
fn reset_mip_generation_jobs(mut mip_generation_jobs: ResMut<MipGenerationJobs>) {
    mip_generation_jobs.clear();
}

/// Returns true if the current platform can use a single bind group for
/// single-pass downsampling.
///
/// If this platform must use two separate bind groups, one for each pass, this
/// function returns false.
pub fn can_combine_downsampling_bind_groups(
    render_adapter: &RenderAdapter,
    render_device: &RenderDevice,
) -> bool {
    // Determine whether we can use a single, large bind group for all mip outputs
    let storage_texture_limit = render_device.limits().max_storage_textures_per_shader_stage;

    // Determine whether we can read and write to the same rgba16f storage texture
    let read_write_support = render_adapter
        .get_texture_format_features(TextureFormat::Rgba16Float)
        .flags
        .contains(TextureFormatFeatureFlags::STORAGE_READ_WRITE);

    // Combine the bind group and use read-write storage if it is supported
    storage_texture_limit >= REQUIRED_STORAGE_TEXTURES && read_write_support
}
