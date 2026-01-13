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
use bevy_asset::{embedded_asset, load_embedded_asset, AssetId, Assets, Handle};
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
        binding_types::uniform_buffer, BindGroupLayoutDescriptor, FilterMode, ShaderType,
        TextureFormatFeatureFlags, UniformBuffer,
    },
    renderer::{RenderAdapter, RenderQueue},
    settings::WgpuFeatures,
    texture::GpuImage,
    RenderStartup,
};
use bevy_render::{
    render_resource::{
        binding_types::{sampler, texture_2d, texture_storage_2d},
        BindGroup, BindGroupEntries, BindGroupLayoutEntries, CachedComputePipelineId,
        ComputePassDescriptor, ComputePipelineDescriptor, Extent3d, PipelineCache, Sampler,
        SamplerBindingType, SamplerDescriptor, ShaderStages, SpecializedComputePipelines,
        StorageTextureAccess, TextureAspect, TextureDescriptor, TextureDimension, TextureFormat,
        TextureUsages, TextureView, TextureViewDescriptor, TextureViewDimension,
    },
    renderer::{RenderContext, RenderDevice},
    Render, RenderApp, RenderSystems,
};
use bevy_shader::{Shader, ShaderDefVal};
use bevy_utils::default;

pub mod experimental;

/// A resource that stores the shaders that perform downsampling.
#[derive(Clone, Resource)]
pub struct DownsampleShaders {
    /// The experimental shader that downsamples depth
    /// (`downsample_depth.wgsl`).
    pub depth: Handle<Shader>,
    /// The shaders that perform downsampling of color textures.
    ///
    /// This table maps a [`TextureFormat`] to the shader that performs
    /// downsampling for textures in that format.
    pub general: HashMap<TextureFormat, Handle<Shader>>,
}

// The number of storage textures required to combine the bind groups in the
// downsampling shader.
const REQUIRED_STORAGE_TEXTURES: u32 = 12;

/// All texture formats that we can perform downsampling for.
///
/// This is a list of pairs, each of which consists of the [`TextureFormat`] and
/// the WGSL name for that texture format.
///
/// The comprehensive list of WGSL names for texture formats can be found in
/// [the relevant section of the WGSL specification].
///
/// [the relevant section of the WGSL specification]:
/// https://www.w3.org/TR/WGSL/#texel-formats
static TEXTURE_FORMATS: [(TextureFormat, &str); 40] = [
    (TextureFormat::Rgba8Unorm, "rgba8unorm"),
    (TextureFormat::Rgba8Snorm, "rgba8snorm"),
    (TextureFormat::Rgba8Uint, "rgba8uint"),
    (TextureFormat::Rgba8Sint, "rgba8sint"),
    (TextureFormat::Rgba16Unorm, "rgba16unorm"),
    (TextureFormat::Rgba16Snorm, "rgba16snorm"),
    (TextureFormat::Rgba16Uint, "rgba16uint"),
    (TextureFormat::Rgba16Sint, "rgba16sint"),
    (TextureFormat::Rgba16Float, "rgba16float"),
    (TextureFormat::Rg8Unorm, "rg8unorm"),
    (TextureFormat::Rg8Snorm, "rg8snorm"),
    (TextureFormat::Rg8Uint, "rg8uint"),
    (TextureFormat::Rg8Sint, "rg8sint"),
    (TextureFormat::Rg16Unorm, "rg16unorm"),
    (TextureFormat::Rg16Snorm, "rg16snorm"),
    (TextureFormat::Rg16Uint, "rg16uint"),
    (TextureFormat::Rg16Sint, "rg16sint"),
    (TextureFormat::Rg16Float, "rg16float"),
    (TextureFormat::R32Uint, "r32uint"),
    (TextureFormat::R32Sint, "r32sint"),
    (TextureFormat::R32Float, "r32float"),
    (TextureFormat::Rg32Uint, "rg32uint"),
    (TextureFormat::Rg32Sint, "rg32sint"),
    (TextureFormat::Rg32Float, "rg32float"),
    (TextureFormat::Rgba32Uint, "rgba32uint"),
    (TextureFormat::Rgba32Sint, "rgba32sint"),
    (TextureFormat::Rgba32Float, "rgba32float"),
    (TextureFormat::Bgra8Unorm, "bgra8unorm"),
    (TextureFormat::R8Unorm, "r8unorm"),
    (TextureFormat::R8Snorm, "r8snorm"),
    (TextureFormat::R8Uint, "r8uint"),
    (TextureFormat::R8Sint, "r8sint"),
    (TextureFormat::R16Unorm, "r16unorm"),
    (TextureFormat::R16Snorm, "r16snorm"),
    (TextureFormat::R16Uint, "r16uint"),
    (TextureFormat::R16Sint, "r16sint"),
    (TextureFormat::R16Float, "r16float"),
    (TextureFormat::Rgb10a2Unorm, "rgb10a2unorm"),
    (TextureFormat::Rgb10a2Uint, "rgb10a2uint"),
    (TextureFormat::Rg11b10Ufloat, "rg11b10ufloat"),
];

/// A render-world resource that stores a list of [`Image`]s that will have
/// mipmaps generated for them.
///
/// You can add images to this list via the [`MipGenerationJobs::add`] method,
/// in the render world. Note that this, by itself, isn't enough to generate
/// the mipmaps; you must also add a [`MipGenerationNode`] to the render graph.
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
    /// [`MipGenerationNode`] corresponding to the [`MipGenerationPhaseId`].
    /// Note that, by default, Bevy doesn't automatically add any such node to
    /// the render graph; it's up to you to manually add that node.
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

/// Stores all render pipelines and bind groups associated with the mipmap
/// generation shader.
///
/// The `prepare_mip_generator_pipelines` system populates this resource lazily
/// as new textures are scheduled.
#[derive(Resource, Default)]
pub struct MipGenerationPipelines {
    /// The pipeline for each texture format.
    ///
    /// Note that pipelines can be shared among all images that use a single
    /// texture format.
    pub(crate) pipelines: HashMap<TextureFormat, MipGenerationTextureFormatPipelines>,

    /// The bind group for each image.
    ///
    /// These are cached from frame to frame if the same image needs mips
    /// generated for it on immediately-consecutive frames.
    pub(crate) bind_groups: HashMap<AssetId<Image>, MipGenerationJobBindGroups>,
}

/// The compute pipelines and bind group layouts for the single-pass
/// downsampling shader for a single texture format.
///
/// Note that, despite the name, the single-pass downsampling shader has two
/// passes, not one. This is because WGSL doesn't presently support
/// globally-coherent buffers; the only way to have a synchronization point is
/// to issue a second dispatch.
struct MipGenerationTextureFormatPipelines {
    /// The bind group layout for the first pass of the downsampling shader.
    downsampling_bind_group_layout_pass_1: BindGroupLayoutDescriptor,
    /// The bind group layout for the second pass of the downsampling shader.
    downsampling_bind_group_layout_pass_2: BindGroupLayoutDescriptor,
    /// The compute pipeline for the first pass of the downsampling shader.
    downsampling_pipeline_pass_1: CachedComputePipelineId,
    /// The compute pipeline for the second pass of the downsampling shader.
    downsampling_pipeline_pass_2: CachedComputePipelineId,
}

/// Bind groups for the downsampling shader associated with a single texture.
struct MipGenerationJobBindGroups {
    /// The bind group for the first downsampling compute pass.
    downsampling_bind_group_pass_1: BindGroup,
    /// The bind group for the second downsampling compute pass.
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
        embedded_asset!(app, "experimental/downsample_depth.wgsl");
        embedded_asset!(app, "downsample.wgsl");

        let depth_shader = load_embedded_asset!(app, "experimental/downsample_depth.wgsl");

        // We don't have string-valued shader definitions in `naga_oil`, so we
        // use a text-pasting hack. The `downsample.wgsl` shader is eagerly
        // specialized for each texture format by replacing `##TEXTURE_FORMAT##`
        // with each possible format.
        // When we have WESL, we should probably revisit this.
        let mut shader_assets = app.world_mut().resource_mut::<Assets<Shader>>();
        let shader_template_source = include_str!("downsample.wgsl");
        let general_shaders: HashMap<_, _> = TEXTURE_FORMATS
            .iter()
            .map(|(texture_format, identifier)| {
                let shader_source =
                    shader_template_source.replace("##TEXTURE_FORMAT##", identifier);
                (
                    *texture_format,
                    shader_assets.add(Shader::from_wgsl(shader_source, "downsample.wgsl")),
                )
            })
            .collect();

        let downsample_shaders = DownsampleShaders {
            depth: depth_shader,
            general: general_shaders,
        };
        app.insert_resource(downsample_shaders.clone());

        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        render_app
            .init_resource::<SpecializedComputePipelines<DownsampleDepthPipeline>>()
            .init_resource::<MipGenerationJobs>()
            .init_resource::<MipGenerationPipelines>()
            .insert_resource(downsample_shaders)
            .add_systems(RenderStartup, depth::init_depth_pyramid_dummy_texture)
            .add_systems(
                Core3d,
                (
                    early_downsample_depth
                        .after(early_deferred_prepass)
                        .before(late_prepass),
                    late_downsample_depth
                        .after(Core3dSystems::StartMainPassPostProcessing)
                        .before(Core3dSystems::EndMainPassPostProcessing),
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

    fn finish(&self, app: &mut App) {
        let Some(render_app) = app.get_sub_app_mut(RenderApp) else {
            return;
        };

        // This needs to be done here so that we have access to the
        // `RenderDevice`.
        render_app.init_resource::<MipGenerationResources>();
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
                mipmap_filter: FilterMode::Nearest,
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
    mip_generation_bind_groups: &MipGenerationPipelines,
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
        let Some(mip_generation_job_bind_groups) = mip_generation_bind_groups
            .bind_groups
            .get(mip_generation_job)
        else {
            continue;
        };
        let Some(mip_generation_pipelines) = mip_generation_bind_groups
            .pipelines
            .get(&gpu_image.texture_format)
        else {
            continue;
        };

        // Fetch the mip generation pipelines.
        let (Some(mip_generation_pipeline_pass_1), Some(mip_generation_pipeline_pass_2)) = (
            pipeline_cache
                .get_compute_pipeline(mip_generation_pipelines.downsampling_pipeline_pass_1),
            pipeline_cache
                .get_compute_pipeline(mip_generation_pipelines.downsampling_pipeline_pass_2),
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
            compute_pass_1.set_bind_group(
                0,
                &mip_generation_job_bind_groups.downsampling_bind_group_pass_1,
                &[],
            );
            compute_pass_1.dispatch_workgroups(
                gpu_image.size.width.div_ceil(64),
                gpu_image.size.height.div_ceil(64),
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
            compute_pass_2.set_bind_group(
                0,
                &mip_generation_job_bind_groups.downsampling_bind_group_pass_2,
                &[],
            );
            compute_pass_2.dispatch_workgroups(
                gpu_image.size.width.div_ceil(256),
                gpu_image.size.height.div_ceil(256),
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
    mip_generation_bind_groups: ResMut<MipGenerationPipelines>,
    mip_generation_resources: Res<MipGenerationResources>,
    mip_generation_jobs: Res<MipGenerationJobs>,
    pipeline_cache: Res<PipelineCache>,
    gpu_images: Res<RenderAssets<GpuImage>>,
    downsample_shaders: Res<DownsampleShaders>,
    render_adapter: Res<RenderAdapter>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
) {
    let mip_generation_pipelines = mip_generation_bind_groups.into_inner();

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

            // Create pipelines for this texture format if necessary. We have at
            // most one pipeline per texture format, regardless of the number of
            // jobs that use that texture format that there are.
            let Some(pipelines) = get_or_create_mip_generation_pipelines(
                &render_device,
                &pipeline_cache,
                &downsample_shaders,
                &mut mip_generation_pipelines.pipelines,
                gpu_image.texture_format,
                mip_generation_job,
                combine_downsampling_bind_groups,
            ) else {
                continue;
            };

            // Create bind groups for the job if necessary.

            let Entry::Vacant(vacant_entry) = mip_generation_pipelines
                .bind_groups
                .entry(*mip_generation_job)
            else {
                continue;
            };

            let downsampling_constants_buffer =
                create_downsampling_constants_buffer(&render_device, &render_queue, gpu_image);

            let (downsampling_bind_group_pass_1, downsampling_bind_group_pass_2) =
                create_downsampling_bind_groups(
                    &render_device,
                    &pipeline_cache,
                    &mip_generation_resources,
                    &downsampling_constants_buffer,
                    pipelines,
                    gpu_image,
                    combine_downsampling_bind_groups,
                );

            vacant_entry.insert(MipGenerationJobBindGroups {
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
        .bind_groups
        .retain(|asset_id, _| all_source_images.contains(asset_id));
}

/// Returns the [`MipGenerationTextureFormatPipelines`] for a single texture
/// format, creating it if necessary.
///
/// The [`MipGenerationTextureFormatPipelines`] that this function returns
/// contains both the bind group layouts and pipelines for all invocations of
/// the single-pass downsampling shader. Note that all images that share a
/// texture format can share the same [`MipGenerationTextureFormatPipelines`]
/// instance.
fn get_or_create_mip_generation_pipelines<'a>(
    render_device: &RenderDevice,
    pipeline_cache: &PipelineCache,
    downsample_shaders: &DownsampleShaders,
    mip_generation_pipelines: &'a mut HashMap<TextureFormat, MipGenerationTextureFormatPipelines>,
    texture_format: TextureFormat,
    mip_generation_job: &AssetId<Image>,
    combine_downsampling_bind_groups: bool,
) -> Option<&'a MipGenerationTextureFormatPipelines> {
    match mip_generation_pipelines.entry(texture_format) {
        Entry::Vacant(vacant_entry) => {
            let Some(downsample_shader) = downsample_shaders.general.get(&texture_format) else {
                error!(
                    "Attempted to generate mips for texture {:?} with format {:?}, but no \
                     downsample shader was available for that texture format",
                    mip_generation_job, texture_format
                );
                return None;
            };

            let (downsampling_bind_group_layout_pass_1, downsampling_bind_group_layout_pass_2) =
                create_downsampling_bind_group_layouts(
                    texture_format,
                    combine_downsampling_bind_groups,
                );

            let (downsampling_pipeline_pass_1, downsampling_pipeline_pass_2) =
                create_downsampling_pipelines(
                    render_device,
                    pipeline_cache,
                    &downsampling_bind_group_layout_pass_1,
                    &downsampling_bind_group_layout_pass_2,
                    downsample_shader,
                    texture_format,
                    combine_downsampling_bind_groups,
                );

            Some(vacant_entry.insert(MipGenerationTextureFormatPipelines {
                downsampling_bind_group_layout_pass_1,
                downsampling_bind_group_layout_pass_2,
                downsampling_pipeline_pass_1,
                downsampling_pipeline_pass_2,
            }))
        }

        Entry::Occupied(occupied_entry) => Some(occupied_entry.into_mut()),
    }
}

/// Creates the [`BindGroupLayoutDescriptor`]s for the single-pass downsampling
/// shader for a single texture format.
fn create_downsampling_bind_group_layouts(
    texture_format: TextureFormat,
    combine_downsampling_bind_groups: bool,
) -> (BindGroupLayoutDescriptor, BindGroupLayoutDescriptor) {
    let texture_sample_type = texture_format.sample_type(None, None).expect(
        "Depth and multisample texture formats shouldn't have mip generation shaders to begin with",
    );
    let mips_storage = texture_storage_2d(texture_format, StorageTextureAccess::WriteOnly);

    if combine_downsampling_bind_groups {
        let bind_group_layout_descriptor = BindGroupLayoutDescriptor::new(
            "combined mip generation bind group layout",
            &BindGroupLayoutEntries::sequential(
                ShaderStages::COMPUTE,
                (
                    sampler(SamplerBindingType::Filtering),
                    uniform_buffer::<DownsamplingConstants>(false),
                    texture_2d(texture_sample_type),
                    mips_storage, // 1
                    mips_storage, // 2
                    mips_storage, // 3
                    mips_storage, // 4
                    mips_storage, // 5
                    texture_storage_2d(texture_format, StorageTextureAccess::ReadWrite), // 6
                    mips_storage, // 7
                    mips_storage, // 8
                    mips_storage, // 9
                    mips_storage, // 10
                    mips_storage, // 11
                    mips_storage, // 12
                ),
            ),
        );
        return (
            bind_group_layout_descriptor.clone(),
            bind_group_layout_descriptor,
        );
    }

    // If we got here, we use a split layout. The first pass outputs mip levels
    // [0, 6]; the second pass outputs mip levels [7, 12].

    let bind_group_layout_descriptor_pass_1 = BindGroupLayoutDescriptor::new(
        "mip generation bind group layout, pass 1",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<DownsamplingConstants>(false),
                // Input mip 0
                texture_2d(texture_sample_type),
                mips_storage, // 1
                mips_storage, // 2
                mips_storage, // 3
                mips_storage, // 4
                mips_storage, // 5
                mips_storage, // 6
            ),
        ),
    );

    let bind_group_layout_descriptor_pass_2 = BindGroupLayoutDescriptor::new(
        "mip generation bind group layout, pass 2",
        &BindGroupLayoutEntries::sequential(
            ShaderStages::COMPUTE,
            (
                sampler(SamplerBindingType::Filtering),
                uniform_buffer::<DownsamplingConstants>(false),
                // Input mip 6
                texture_2d(texture_sample_type),
                mips_storage, // 7
                mips_storage, // 8
                mips_storage, // 9
                mips_storage, // 10
                mips_storage, // 11
                mips_storage, // 12
            ),
        ),
    );

    (
        bind_group_layout_descriptor_pass_1,
        bind_group_layout_descriptor_pass_2,
    )
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
    pipelines: &MipGenerationTextureFormatPipelines,
    gpu_image: &GpuImage,
    combine_downsampling_bind_groups: bool,
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
    if combine_downsampling_bind_groups {
        let bind_group = render_device.create_bind_group(
            Some("combined mip generation bind group"),
            &pipeline_cache.get_bind_group_layout(&pipelines.downsampling_bind_group_layout_pass_1),
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
        base_mip_level: gpu_image.mip_level_count.min(6),
        mip_level_count: Some(1),
        ..default()
    });

    let bind_group_pass_1 = render_device.create_bind_group(
        "mip generation bind group, pass 1",
        &pipeline_cache.get_bind_group_layout(&pipelines.downsampling_bind_group_layout_pass_1),
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
        &pipeline_cache.get_bind_group_layout(&pipelines.downsampling_bind_group_layout_pass_2),
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

/// Creates the single-pass downsampling compute pipelines that perform
/// downsampling on textures with a specific texture format.
///
/// Depending on whether the current platform can combine downsampling bind
/// groups, this will either return two copies of the same pipeline or two
/// different pipelines.
fn create_downsampling_pipelines(
    render_device: &RenderDevice,
    pipeline_cache: &PipelineCache,
    downsampling_bind_group_layout_pass_1: &BindGroupLayoutDescriptor,
    downsampling_bind_group_layout_pass_2: &BindGroupLayoutDescriptor,
    downsample_shader: &Handle<Shader>,
    texture_format: TextureFormat,
    combine_downsampling_bind_groups: bool,
) -> (CachedComputePipelineId, CachedComputePipelineId) {
    let mut downsampling_shader_defs = vec![];
    if render_device.features().contains(WgpuFeatures::SUBGROUP) {
        downsampling_shader_defs.push(ShaderDefVal::Int("SUBGROUP_SUPPORT".into(), 1));
    }
    if combine_downsampling_bind_groups {
        downsampling_shader_defs.push(ShaderDefVal::Int("COMBINE_BIND_GROUP".into(), 1));
    }

    let mut downsampling_first_shader_defs = downsampling_shader_defs.clone();
    let mut downsampling_second_shader_defs = downsampling_shader_defs.clone();
    if !combine_downsampling_bind_groups {
        downsampling_first_shader_defs.push(ShaderDefVal::Int("FIRST_PASS".into(), 1));
        downsampling_second_shader_defs.push(ShaderDefVal::Int("SECOND_PASS".into(), 1));
    }

    // Create the pipeline for the first pass, corresponding to mip levels [0,
    // 6].
    let downsampling_first_pipeline =
        pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(format!("mip generation pipeline, pass 1 ({:?})", texture_format).into()),
            layout: vec![downsampling_bind_group_layout_pass_1.clone()],
            push_constant_ranges: vec![],
            shader: downsample_shader.clone(),
            shader_defs: downsampling_first_shader_defs,
            entry_point: Some("downsample_first".into()),
            zero_initialize_workgroup_memory: false,
        });

    // Create the pipeline for the second pass, corresponding to mip levels [7,
    // 12].
    let downsampling_second_pipeline =
        pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some(format!("mip generation pipeline, pass 2 ({:?})", texture_format).into()),
            layout: vec![downsampling_bind_group_layout_pass_2.clone()],
            push_constant_ranges: vec![],
            shader: downsample_shader.clone(),
            shader_defs: downsampling_second_shader_defs,
            entry_point: Some("downsample_second".into()),
            zero_initialize_workgroup_memory: false,
        });

    (downsampling_first_pipeline, downsampling_second_pipeline)
}

/// Creates the uniform buffer containing the [`DownsamplingConstants`] for a
/// single texture.
fn create_downsampling_constants_buffer(
    render_device: &RenderDevice,
    render_queue: &RenderQueue,
    gpu_image: &GpuImage,
) -> UniformBuffer<DownsamplingConstants> {
    let downsampling_constants = DownsamplingConstants {
        mips: gpu_image.mip_level_count,
        inverse_input_size: vec2(
            1.0 / gpu_image.size.width as f32,
            1.0 / gpu_image.size.height as f32,
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
    if level < gpu_image.mip_level_count {
        return gpu_image.texture.create_view(&TextureViewDescriptor {
            label: Some(&*format!(
                "mip downsampling storage view {}/{}",
                level, gpu_image.mip_level_count
            )),
            format: Some(gpu_image.texture_format),
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
            level, gpu_image.mip_level_count
        )),
        size: Extent3d {
            width: 1,
            height: 1,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: gpu_image.texture_format,
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
