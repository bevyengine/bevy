use crate::{
    curl_noise::CurlNoiseTexture, detail_noise::DetailNoiseTexture, fbm_noise::FbmNoiseTexture, perlin_worley_noise::PerlinWorleyNoiseTexture, Bluenoise,
    CloudLayer, ExtractedAtmosphere, ExtractedDirectionalLight, GpuLights, GpuScatteringMedium,
    LightMeta, ScatteringMediumSampler, ShadowSamplers, ViewShadowBindings,
};
use bevy_asset::{load_embedded_asset, AssetId, Handle};
use bevy_platform::collections::HashMap;
use bevy_render::camera::ExtractedCamera;
use bevy_render::render_resource::UniformBuffer;
use bevy_render::view::RetainedViewEntity;
use bevy_camera::{Camera, Camera3d};
use bevy_core_pipeline::FullscreenShader;
use bevy_derive::Deref;
use bevy_diagnostic::FrameCount;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    error::BevyError,
    query::With,
    resource::Resource,
    system::{Commands, Query, Res, ResMut},
    world::{FromWorld, World},
};
use bevy_image::ToExtents;
use bevy_light::atmosphere::ScatteringMedium;
use bevy_math::{Affine3A, Mat4, UVec2, Vec2, Vec3, Vec3A};
use bevy_render::{
    extract_component::ComponentUniforms,
    render_asset::RenderAssets,
    render_resource::{binding_types::*, TextureViewDescriptor, TextureViewDimension, *},
    renderer::{RenderDevice, RenderQueue},
    texture::{CachedTexture, FallbackImage, GpuImage, TextureCache},
    view::{ExtractedView, Msaa, ViewDepthStencilTexture, ViewUniform, ViewUniforms},
};
use bevy_shader::Shader;
use bevy_utils::default;

use super::GpuAtmosphereSettings;

/// Shadow map bind group layout entries
fn shadow_map_layout_entries() -> impl IntoIndexedBindGroupLayoutEntryBuilderArray<4> {
    (
        (
            14,
            #[cfg(all(
                not(target_abi = "sim"),
                any(
                    not(feature = "webgl"),
                    not(target_arch = "wasm32"),
                    feature = "webgpu"
                )
            ))]
            texture_cube_array(TextureSampleType::Depth),
            #[cfg(any(
                target_abi = "sim",
                all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu"))
            ))]
            texture_cube(TextureSampleType::Depth),
        ),
        (15, sampler(SamplerBindingType::Comparison)),
        (
            16,
            #[cfg(any(
                not(feature = "webgl"),
                not(target_arch = "wasm32"),
                feature = "webgpu"
            ))]
            texture_2d_array(TextureSampleType::Depth),
            #[cfg(all(feature = "webgl", target_arch = "wasm32", not(feature = "webgpu")))]
            texture_2d(TextureSampleType::Depth),
        ),
        (17, sampler(SamplerBindingType::Comparison)),
    )
}

/// Cloud bind group layout entries (cloud layer uniform, noise textures, cloud shadow map, STBN).
/// Sampled outputs of the `clouds_raymarch` pass, read by `render_sky` (26: luminance and
/// transmittance, 27: transmittance-weighted cloud depth).
fn cloud_pass_output_entries() -> impl IntoIndexedBindGroupLayoutEntryBuilderArray<2> {
    (
        (26, texture_2d(TextureSampleType::Float { filterable: true })),
        (27, texture_2d(TextureSampleType::Float { filterable: false })),
    )
}

/// Placed at 20..=25, 33..=34 so they don't collide with the shadow map entries (14..=17)
/// or the clouds_reconstruct history/storage bindings (28..=32).
fn cloud_layout_entries() -> impl IntoIndexedBindGroupLayoutEntryBuilderArray<8> {
    (
        (20, uniform_buffer::<CloudLayer>(true)), // cloud layer parameters
        // 2D noise texture for cloud coverage / weather
        (
            21,
            texture_2d(TextureSampleType::Float { filterable: true }),
        ),
        (22, sampler(SamplerBindingType::Filtering)), // noise sampler
        // cloud shadow map
        (
            23,
            texture_2d(TextureSampleType::Float { filterable: true }),
        ),
        // 3D Perlin–Worley shape noise (128³ + mips)
        (
            24,
            texture_3d(TextureSampleType::Float { filterable: true }),
        ),
        // STBN for temporal stratification (different layer per frame)
        (
            25,
            texture_2d_array(TextureSampleType::Float { filterable: true }),
        ),
        // 3D Worley detail / erosion noise (32³ + mips)
        (
            33,
            texture_3d(TextureSampleType::Float { filterable: true }),
        ),
        // 2D curl noise for distortion (128²)
        (
            34,
            texture_2d(TextureSampleType::Float { filterable: true }),
        ),
    )
}

#[derive(Resource)]
pub(crate) struct AtmosphereBindGroupLayouts {
    pub transmittance_lut: BindGroupLayoutDescriptor,
    pub multiscattering_lut: BindGroupLayoutDescriptor,
    pub sky_view_lut: BindGroupLayoutDescriptor,
    pub aerial_view_lut: BindGroupLayoutDescriptor,
    pub cloud_shadow_map: BindGroupLayoutDescriptor,
    // Accessed from the render-world graph; some tooling can miss the cross-module usage.
    #[allow(dead_code)]
    pub cloud_shadow_filter: BindGroupLayoutDescriptor,
    pub cloud_shadow_temporal: BindGroupLayoutDescriptor,
    pub clouds_raymarch: BindGroupLayoutDescriptor,
    pub clouds_reconstruct: BindGroupLayoutDescriptor,
}

#[derive(Resource)]
pub(crate) struct RenderSkyBindGroupLayouts {
    // Two variants:
    // - *_clouds: includes CloudLayer + cloud noise + cloud shadow map bindings
    // - *_no_clouds: omits cloud-related bindings (cheaper, and works when CloudLayer is removed)
    pub render_sky_clouds: BindGroupLayoutDescriptor,
    pub render_sky_msaa_clouds: BindGroupLayoutDescriptor,
    pub render_sky_no_clouds: BindGroupLayoutDescriptor,
    pub render_sky_msaa_no_clouds: BindGroupLayoutDescriptor,
    pub fullscreen_shader: FullscreenShader,
    pub fragment_shader: Handle<Shader>,
}

impl AtmosphereBindGroupLayouts {
    pub fn new() -> Self {
        let transmittance_lut = BindGroupLayoutDescriptor::new(
            "transmittance_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    // scattering medium luts and sampler
                    (5, texture_2d(TextureSampleType::default())),
                    (6, texture_2d(TextureSampleType::default())),
                    (7, sampler(SamplerBindingType::Filtering)),
                    // transmittance lut storage texture
                    (
                        13,
                        texture_storage_2d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            ),
        );

        let multiscattering_lut = BindGroupLayoutDescriptor::new(
            "multiscattering_lut_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    // scattering medium luts and sampler
                    (5, texture_2d(TextureSampleType::default())),
                    (6, texture_2d(TextureSampleType::default())),
                    (7, sampler(SamplerBindingType::Filtering)),
                    // atmosphere luts and sampler
                    (8, texture_2d(TextureSampleType::default())), // transmittance
                    (12, sampler(SamplerBindingType::Filtering)),
                    // multiscattering lut storage texture
                    (
                        13,
                        texture_storage_2d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            ),
        );

        let sky_view_lut_entries = DynamicBindGroupLayoutEntries::new_with_indices(
            ShaderStages::COMPUTE,
            (
                (0, uniform_buffer::<GpuAtmosphere>(true)),
                (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                (2, uniform_buffer::<AtmosphereTransform>(true)),
                (3, uniform_buffer::<ViewUniform>(true)),
                (4, uniform_buffer::<GpuLights>(true)),
                // scattering medium luts and sampler
                (5, texture_2d(TextureSampleType::default())),
                (6, texture_2d(TextureSampleType::default())),
                (7, sampler(SamplerBindingType::Filtering)),
                // atmosphere luts and sampler
                (8, texture_2d(TextureSampleType::default())), // transmittance
                (9, texture_2d(TextureSampleType::default())), // multiscattering
                (12, sampler(SamplerBindingType::Filtering)),
                // sky view lut storage texture
                (
                    13,
                    texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        )
        .extend_with_indices(shadow_map_layout_entries());
        let sky_view_lut =
            BindGroupLayoutDescriptor::new("sky_view_lut_bind_group_layout", &sky_view_lut_entries);

        let aerial_view_lut_entries = DynamicBindGroupLayoutEntries::new_with_indices(
            ShaderStages::COMPUTE,
            (
                (0, uniform_buffer::<GpuAtmosphere>(true)),
                (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                (3, uniform_buffer::<ViewUniform>(true)),
                (4, uniform_buffer::<GpuLights>(true)),
                // scattering medium luts and sampler
                (5, texture_2d(TextureSampleType::default())),
                (6, texture_2d(TextureSampleType::default())),
                (7, sampler(SamplerBindingType::Filtering)),
                // atmosphere luts and sampler
                (8, texture_2d(TextureSampleType::default())), // transmittance
                (9, texture_2d(TextureSampleType::default())), // multiscattering
                (12, sampler(SamplerBindingType::Filtering)),
                // aerial view lut storage texture
                (
                    13,
                    texture_storage_3d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                ),
            ),
        )
        .extend_with_indices(shadow_map_layout_entries());
        let aerial_view_lut = BindGroupLayoutDescriptor::new(
            "aerial_view_lut_bind_group_layout",
            &aerial_view_lut_entries,
        );

        let cloud_shadow_map = BindGroupLayoutDescriptor::new(
            "cloud_shadow_map_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, uniform_buffer::<GpuLights>(true)),
                    (20, uniform_buffer::<CloudLayer>(true)),
                    (
                        21,
                        texture_2d(TextureSampleType::Float { filterable: true }),
                    ),
                    (22, sampler(SamplerBindingType::Filtering)),
                    // 3D Perlin–Worley shape noise
                    (
                        24,
                        texture_3d(TextureSampleType::Float { filterable: true }),
                    ),
                    // STBN for temporal stratification (different layer per frame)
                    (
                        25,
                        texture_2d_array(TextureSampleType::Float { filterable: true }),
                    ),
                    // detail Worley + curl
                    (
                        33,
                        texture_3d(TextureSampleType::Float { filterable: true }),
                    ),
                    (
                        34,
                        texture_2d(TextureSampleType::Float { filterable: true }),
                    ),
                    (
                        13,
                        texture_storage_2d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            ),
        );

        // Cloud shadow spatial filter (ping-pong).
        // Reads: sampled shadow map (rgba16f)
        // Writes: storage shadow map (rgba16f)
        let cloud_shadow_filter = BindGroupLayoutDescriptor::new(
            "cloud_shadow_filter_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    (12, sampler(SamplerBindingType::Filtering)),
                    (
                        23,
                        texture_2d(TextureSampleType::Float { filterable: true }),
                    ),
                    (
                        13,
                        texture_storage_2d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            ),
        );

        // Cloud shadow temporal filter.
        // Reads: traced output (storage), history (sampled)
        // Writes: output (storage)
        let cloud_shadow_temporal = BindGroupLayoutDescriptor::new(
            "cloud_shadow_temporal_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<CloudShadowTemporalParams>(false)),
                    (
                        1,
                        texture_storage_2d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::ReadOnly,
                        ),
                    ),
                    (2, texture_2d(TextureSampleType::Float { filterable: true })),
                    (3, sampler(SamplerBindingType::Filtering)),
                    (
                        4,
                        texture_storage_2d(
                            TextureFormat::Rgba16Float,
                            StorageTextureAccess::WriteOnly,
                        ),
                    ),
                ),
            ),
        );

        // Dedicated low-res cloud raymarch (`clouds_raymarch.wesl`).
        // Same bindings as `render_sky` (clouds variant, non-MSAA depth, no shadow maps) plus the
        // storage outputs at 26 (rgba16f luminance/transmittance) and 27 (r32f weighted depth).
        let clouds_raymarch = BindGroupLayoutDescriptor::new(
            "clouds_raymarch_bind_group_layout",
            &DynamicBindGroupLayoutEntries::new_with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    (2, uniform_buffer::<AtmosphereTransform>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, uniform_buffer::<GpuLights>(true)),
                    (5, texture_2d(TextureSampleType::default())),
                    (6, texture_2d(TextureSampleType::default())),
                    (7, sampler(SamplerBindingType::Filtering)),
                    (8, texture_2d(TextureSampleType::default())),
                    (9, texture_2d(TextureSampleType::default())),
                    (10, texture_2d(TextureSampleType::default())),
                    (11, texture_3d(TextureSampleType::default())),
                    (12, sampler(SamplerBindingType::Filtering)),
                    (13, texture_2d(TextureSampleType::Depth)),
                ),
            )
            .extend_with_indices(cloud_layout_entries())
            .extend_with_indices((
                (
                    26,
                    texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                ),
                (
                    27,
                    texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                ),
            )),
        );

        // Checkerboard temporal reconstruction (`clouds_reconstruct.wesl`).
        let clouds_reconstruct = BindGroupLayoutDescriptor::new(
            "clouds_reconstruct_bind_group_layout",
            &BindGroupLayoutEntries::with_indices(
                ShaderStages::COMPUTE,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (12, sampler(SamplerBindingType::Filtering)),
                    (26, texture_2d(TextureSampleType::Float { filterable: true })),
                    (27, texture_2d(TextureSampleType::Float { filterable: false })),
                    (28, texture_2d(TextureSampleType::Float { filterable: true })),
                    (29, texture_2d(TextureSampleType::Float { filterable: false })),
                    (30, uniform_buffer::<CloudsTemporalParams>(false)),
                    (
                        31,
                        texture_storage_2d(TextureFormat::Rgba16Float, StorageTextureAccess::WriteOnly),
                    ),
                    (
                        32,
                        texture_storage_2d(TextureFormat::R32Float, StorageTextureAccess::WriteOnly),
                    ),
                ),
            ),
        );

        Self {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
            cloud_shadow_map,
            cloud_shadow_filter,
            cloud_shadow_temporal,
            clouds_raymarch,
            clouds_reconstruct,
        }
    }
}

/// Reprojection parameters for the checkerboarded cloud reconstruction pass.
#[derive(Clone, Copy, ShaderType)]
pub struct CloudsTemporalParams {
    pub prev_clip_from_world: Mat4,
    pub history_valid: u32,
    pub new_sample_weight: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

/// Per-view uniform for [`CloudsTemporalParams`].
#[derive(Component)]
pub struct CloudsTemporalParamsUniform(pub UniformBuffer<CloudsTemporalParams>);

/// Writes [`CloudsTemporalParams`] for every atmosphere view, remembering each view's
/// previous `clip_from_world` (keyed by retained view entity) for history reprojection.
pub(super) fn prepare_clouds_temporal_params(
    views: Query<(Entity, &ExtractedView, &AtmosphereTextures), With<ExtractedAtmosphere>>,
    mut previous: bevy_ecs::system::Local<HashMap<RetainedViewEntity, (Mat4, UVec2)>>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut commands: Commands,
) {
    let mut seen = HashMap::default();
    for (entity, view, textures) in &views {
        let clip_from_world = view.clip_from_world.unwrap_or_else(|| {
            view.clip_from_view * view.world_from_view.to_matrix().inverse()
        });
        let size = textures.clouds.texture.size();
        let size = UVec2::new(size.width, size.height);
        let (prev_clip_from_world, history_valid) =
            match previous.get(&view.retained_view_entity) {
                Some((prev, prev_size)) if *prev_size == size => (*prev, 1),
                _ => (clip_from_world, 0),
            };
        let mut buffer = UniformBuffer::from(CloudsTemporalParams {
            prev_clip_from_world,
            history_valid,
            new_sample_weight: 0.5,
            _pad0: 0.0,
            _pad1: 0.0,
        });
        buffer.write_buffer(&render_device, &render_queue);
        commands
            .entity(entity)
            .insert(CloudsTemporalParamsUniform(buffer));
        seen.insert(view.retained_view_entity, (clip_from_world, size));
    }
    *previous = seen;
}

/// Parameters for the cloud shadow temporal filter pass.
/// Used to reproject history from the previous frame.
#[derive(Clone, Default, ShaderType)]
pub struct CloudShadowTemporalParams {
    pub curr_anchor: Vec3,
    pub curr_light_dir: Vec3,
    pub curr_basis_x: Vec3,
    pub curr_basis_y: Vec3,
    pub prev_anchor: Vec3,
    pub prev_light_dir: Vec3,
    pub prev_basis_x: Vec3,
    pub prev_basis_y: Vec3,
    pub extent: f32,
    pub temporal_alpha: f32,
    pub history_valid: u32,
    pub anchor_moved: u32,
    pub size: UVec2,
    /// Time-slicing tile size (1 = every texel is traced each frame).
    pub interleave: u32,
    /// Frame index used to select the traced texel in each tile (matches `view.frame_count`).
    pub frame_index: u32,
}

/// Per-view state for cloud shadow temporal filtering.
/// Persists between frames to enable history reprojection.
#[derive(Resource, Default)]
pub struct CloudShadowTemporalState {
    pub anchor: Vec3,
    pub light_dir: Vec3,
    pub basis_x: Vec3,
    pub basis_y: Vec3,
    pub initialized: bool,
}

impl FromWorld for RenderSkyBindGroupLayouts {
    fn from_world(world: &mut World) -> Self {
        // Base render-sky entries (0..=13) + shadow map entries (14..=17).
        let base_entries = |depth: BindGroupLayoutEntryBuilder| {
            DynamicBindGroupLayoutEntries::new_with_indices(
                ShaderStages::FRAGMENT,
                (
                    (0, uniform_buffer::<GpuAtmosphere>(true)),
                    (1, uniform_buffer::<GpuAtmosphereSettings>(true)),
                    (2, uniform_buffer::<AtmosphereTransform>(true)),
                    (3, uniform_buffer::<ViewUniform>(true)),
                    (4, uniform_buffer::<GpuLights>(true)),
                    // scattering medium luts and sampler
                    (5, texture_2d(TextureSampleType::default())),
                    (6, texture_2d(TextureSampleType::default())),
                    (7, sampler(SamplerBindingType::Filtering)),
                    // atmosphere luts and sampler
                    (8, texture_2d(TextureSampleType::default())), // transmittance
                    (9, texture_2d(TextureSampleType::default())), // multiscattering
                    (10, texture_2d(TextureSampleType::default())), // sky view
                    (11, texture_3d(TextureSampleType::default())), // aerial view
                    (12, sampler(SamplerBindingType::Filtering)),
                    // view depth texture
                    (13, depth),
                ),
            )
            .extend_with_indices(shadow_map_layout_entries())
        };

        // Two variants:
        // - *_clouds: base + CloudLayer + cloud noise + cloud shadow map + STBN (20..=25)
        // - *_no_clouds: base only
        let render_sky_no_clouds = BindGroupLayoutDescriptor::new(
            "render_sky_bind_group_layout_no_clouds",
            &base_entries(texture_2d(TextureSampleType::Depth)),
        );
        let render_sky_msaa_no_clouds = BindGroupLayoutDescriptor::new(
            "render_sky_msaa_bind_group_layout_no_clouds",
            &base_entries(texture_2d_multisampled(TextureSampleType::Depth)),
        );
        let render_sky_clouds = BindGroupLayoutDescriptor::new(
            "render_sky_bind_group_layout_clouds",
            &base_entries(texture_2d(TextureSampleType::Depth))
                .extend_with_indices(cloud_layout_entries())
                .extend_with_indices(cloud_pass_output_entries()),
        );
        let render_sky_msaa_clouds = BindGroupLayoutDescriptor::new(
            "render_sky_msaa_bind_group_layout_clouds",
            &base_entries(texture_2d_multisampled(TextureSampleType::Depth))
                .extend_with_indices(cloud_layout_entries())
                .extend_with_indices(cloud_pass_output_entries()),
        );

        Self {
            render_sky_clouds,
            render_sky_msaa_clouds,
            render_sky_no_clouds,
            render_sky_msaa_no_clouds,
            fullscreen_shader: world.resource::<FullscreenShader>().clone(),
            fragment_shader: load_embedded_asset!(world, "render_sky.wesl"),
        }
    }
}

#[derive(Resource, Deref)]
pub struct AtmosphereSampler(Sampler);

impl FromWorld for AtmosphereSampler {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self(sampler)
    }
}

#[derive(Resource, Deref)]
pub struct CloudNoiseSampler(Sampler);

impl FromWorld for CloudNoiseSampler {
    fn from_world(world: &mut World) -> Self {
        let render_device = world.resource::<RenderDevice>();

        let sampler = render_device.create_sampler(&SamplerDescriptor {
            address_mode_u: AddressMode::Repeat,
            address_mode_v: AddressMode::Repeat,
            address_mode_w: AddressMode::Repeat,
            mag_filter: FilterMode::Linear,
            min_filter: FilterMode::Linear,
            mipmap_filter: MipmapFilterMode::Nearest,
            ..Default::default()
        });

        Self(sampler)
    }
}

#[derive(Resource)]
pub(crate) struct AtmosphereLutPipelines {
    pub transmittance_lut: CachedComputePipelineId,
    pub multiscattering_lut: CachedComputePipelineId,
    pub sky_view_lut: CachedComputePipelineId,
    pub aerial_view_lut: CachedComputePipelineId,
    pub cloud_shadow_map: CachedComputePipelineId,
    // Accessed from the render-world graph; some tooling can miss the cross-module usage.
    #[allow(dead_code)]
    pub cloud_shadow_filter: CachedComputePipelineId,
    pub cloud_shadow_temporal: CachedComputePipelineId,
    pub clouds_raymarch: CachedComputePipelineId,
    pub clouds_reconstruct: CachedComputePipelineId,
}

impl FromWorld for AtmosphereLutPipelines {
    fn from_world(world: &mut World) -> Self {
        let pipeline_cache = world.resource::<PipelineCache>();
        let layouts = world.resource::<AtmosphereBindGroupLayouts>();

        let transmittance_lut = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("transmittance_lut_pipeline".into()),
            layout: vec![layouts.transmittance_lut.clone()],
            shader: load_embedded_asset!(world, "transmittance_lut.wesl"),
            ..default()
        });

        let multiscattering_lut =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("multi_scattering_lut_pipeline".into()),
                layout: vec![layouts.multiscattering_lut.clone()],
                shader: load_embedded_asset!(world, "multiscattering_lut.wesl"),
                ..default()
            });

        let sky_view_lut = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("sky_view_lut_pipeline".into()),
            layout: vec![layouts.sky_view_lut.clone()],
            shader: load_embedded_asset!(world, "sky_view_lut.wesl"),
            // Note: Clouds disabled for sky_view_lut - too complex for precomputation
            // Clouds are rendered in real-time raymarching instead
            ..default()
        });

        let aerial_view_lut = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("aerial_view_lut_pipeline".into()),
            layout: vec![layouts.aerial_view_lut.clone()],
            shader: load_embedded_asset!(world, "aerial_view_lut.wesl"),
            ..default()
        });

        let cloud_shadow_map = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("cloud_shadow_map_pipeline".into()),
            layout: vec![layouts.cloud_shadow_map.clone()],
            shader: load_embedded_asset!(world, "cloud_shadow_map.wesl"),
            ..default()
        });

        let cloud_shadow_filter =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("cloud_shadow_filter_pipeline".into()),
                layout: vec![layouts.cloud_shadow_filter.clone()],
                shader: load_embedded_asset!(world, "cloud_shadow_filter.wesl"),
                ..default()
            });

        let cloud_shadow_temporal =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("cloud_shadow_temporal_pipeline".into()),
                layout: vec![layouts.cloud_shadow_temporal.clone()],
                shader: load_embedded_asset!(world, "cloud_shadow_temporal.wesl"),
                ..default()
            });

        let clouds_raymarch = pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
            label: Some("clouds_raymarch_pipeline".into()),
            layout: vec![layouts.clouds_raymarch.clone()],
            shader: load_embedded_asset!(world, "clouds_raymarch.wesl"),
            shader_defs: vec![
                "CLOUDS_ENABLED".into(),
                "CLOUD_SHADOW_SIMPLE_SAMPLING".into(),
                "ATMOSPHERE_NO_SHADOW_MAPS".into(),
            ],
            ..default()
        });

        let clouds_reconstruct =
            pipeline_cache.queue_compute_pipeline(ComputePipelineDescriptor {
                label: Some("clouds_reconstruct_pipeline".into()),
                layout: vec![layouts.clouds_reconstruct.clone()],
                shader: load_embedded_asset!(world, "clouds_reconstruct.wesl"),
                shader_defs: vec!["ATMOSPHERE_NO_SHADOW_MAPS".into()],
                ..default()
            });

        Self {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
            cloud_shadow_map,
            cloud_shadow_filter,
            cloud_shadow_temporal,
            clouds_raymarch,
            clouds_reconstruct,
        }
    }
}

#[derive(Component)]
pub(crate) struct RenderSkyPipelineId(pub CachedRenderPipelineId);

#[derive(Copy, Clone, Hash, PartialEq, Eq)]
pub(crate) struct RenderSkyPipelineKey {
    pub msaa_samples: u32,
    pub dual_source_blending: bool,
    pub clouds_enabled: bool,
}

impl SpecializedRenderPipeline for RenderSkyBindGroupLayouts {
    type Key = RenderSkyPipelineKey;

    fn specialize(&self, key: Self::Key) -> RenderPipelineDescriptor {
        let mut shader_defs = Vec::new();

        if key.msaa_samples > 1 {
            shader_defs.push("MULTISAMPLED".into());
        }
        if key.dual_source_blending {
            shader_defs.push("DUAL_SOURCE_BLENDING".into());
        }

        // Enable cloud rendering only when a CloudLayer component is present.
        if key.clouds_enabled {
            shader_defs.push("CLOUDS_ENABLED".into());
            shader_defs.push("CLOUD_SHADOW_SIMPLE_SAMPLING".into());
            // Clouds come from the dedicated `clouds_raymarch` pass, not `raymarch_atmosphere`.
            shader_defs.push("CLOUD_PASS".into());
        }

        let dst_factor = if key.dual_source_blending {
            BlendFactor::Src1
        } else {
            BlendFactor::SrcAlpha
        };

        RenderPipelineDescriptor {
            label: Some(format!("render_sky_pipeline_{}", key.msaa_samples).into()),
            layout: vec![match (key.msaa_samples == 1, key.clouds_enabled) {
                (true, true) => self.render_sky_clouds.clone(),
                (false, true) => self.render_sky_msaa_clouds.clone(),
                (true, false) => self.render_sky_no_clouds.clone(),
                (false, false) => self.render_sky_msaa_no_clouds.clone(),
            }],
            vertex: self.fullscreen_shader.to_vertex_state(),
            fragment: Some(FragmentState {
                shader: self.fragment_shader.clone(),
                shader_defs,
                targets: vec![Some(ColorTargetState {
                    format: TextureFormat::Rgba16Float,
                    blend: Some(BlendState {
                        color: BlendComponent {
                            src_factor: BlendFactor::One,
                            dst_factor,
                            operation: BlendOperation::Add,
                        },
                        alpha: BlendComponent {
                            src_factor: BlendFactor::Zero,
                            dst_factor: BlendFactor::One,
                            operation: BlendOperation::Add,
                        },
                    }),
                    write_mask: ColorWrites::ALL,
                })],
                ..default()
            }),
            multisample: MultisampleState {
                count: key.msaa_samples,
                ..default()
            },
            ..default()
        }
    }
}

pub(super) fn queue_render_sky_pipelines(
    views: Query<(Entity, &Msaa, Option<&CloudLayer>), (With<Camera>, With<ExtractedAtmosphere>)>,
    pipeline_cache: Res<PipelineCache>,
    layouts: Res<RenderSkyBindGroupLayouts>,
    mut specializer: ResMut<SpecializedRenderPipelines<RenderSkyBindGroupLayouts>>,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    for (entity, msaa, cloud_layer) in &views {
        let id = specializer.specialize(
            &pipeline_cache,
            &layouts,
            RenderSkyPipelineKey {
                msaa_samples: msaa.samples(),
                dual_source_blending: render_device
                    .features()
                    .contains(WgpuFeatures::DUAL_SOURCE_BLENDING),
                clouds_enabled: cloud_layer.is_some(),
            },
        );
        commands.entity(entity).insert(RenderSkyPipelineId(id));
    }
}

#[derive(Component)]
pub struct AtmosphereTextures {
    pub transmittance_lut: CachedTexture,
    pub multiscattering_lut: CachedTexture,
    pub sky_view_lut: CachedTexture,
    pub aerial_view_lut: CachedTexture,
    pub cloud_shadow_map: CachedTexture,
    pub cloud_shadow_map_tmp: CachedTexture,
    pub cloud_shadow_map_history: CachedTexture,
    /// Low-res cloud raymarch output: rgb = pre-exposed luminance, a = transmittance.
    pub clouds: CachedTexture,
    /// Low-res transmittance-weighted cloud distance (m).
    pub clouds_depth: CachedTexture,
    /// Previous frame's [`Self::clouds`] / [`Self::clouds_depth`] (ping-pong by frame parity).
    pub clouds_history: CachedTexture,
    pub clouds_depth_history: CachedTexture,
    /// Checkerboard trace targets (1 of each 2x2 half-res pixels per frame).
    pub clouds_trace: CachedTexture,
    pub clouds_trace_depth: CachedTexture,
}

pub(super) fn prepare_atmosphere_textures(
    views: Query<
        (Entity, &GpuAtmosphereSettings, Option<&ExtractedCamera>),
        With<ExtractedAtmosphere>,
    >,
    render_device: Res<RenderDevice>,
    mut texture_cache: ResMut<TextureCache>,
    frame_count: Res<FrameCount>,
    mut commands: Commands,
) {
    for (entity, lut_settings, camera) in &views {
        let transmittance_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("transmittance_lut"),
                size: lut_settings.transmittance_lut_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let multiscattering_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("multiscattering_lut"),
                size: lut_settings.multiscattering_lut_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let sky_view_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("sky_view_lut"),
                size: lut_settings.sky_view_lut_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let aerial_view_lut = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("aerial_view_lut"),
                size: lut_settings.aerial_view_lut_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D3,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let cloud_shadow_map = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("cloud_shadow_map"),
                size: lut_settings.cloud_shadow_map_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING
                    | TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_SRC,
                view_formats: &[],
            },
        );

        let cloud_shadow_map_tmp = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("cloud_shadow_map_tmp"),
                size: lut_settings.cloud_shadow_map_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
                view_formats: &[],
            },
        );

        let cloud_shadow_map_history = texture_cache.get(
            &render_device,
            TextureDescriptor {
                label: Some("cloud_shadow_map_history"),
                size: lut_settings.cloud_shadow_map_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::STORAGE_BINDING
                    | TextureUsages::TEXTURE_BINDING
                    | TextureUsages::COPY_DST,
                view_formats: &[],
            },
        );

        // Half-resolution cloud raymarch targets (design.md §1.2).
        let full_size = camera
            .and_then(|camera| camera.physical_viewport_size)
            .unwrap_or(UVec2::ONE);
        let clouds_size = (full_size / 2).max(UVec2::ONE);
        let trace_size = (clouds_size + UVec2::ONE) / 2;
        let clouds_texture =
            |label: &'static str, format: TextureFormat, size: UVec2, copy: TextureUsages| {
                TextureDescriptor {
                    label: Some(label),
                    size: size.to_extents(),
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: TextureDimension::D2,
                    format,
                    usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING | copy,
                    view_formats: &[],
                }
            };
        let mut get = |label, format, size, copy| {
            texture_cache.get(&render_device, clouds_texture(label, format, size, copy))
        };
        // Ping-pong reconstruction targets: distinct labels keep the cached textures stable
        // across frames, and the frame parity picks which one is written this frame.
        let none = TextureUsages::empty();
        let clouds_a = get("clouds_a", TextureFormat::Rgba16Float, clouds_size, none);
        let clouds_b = get("clouds_b", TextureFormat::Rgba16Float, clouds_size, none);
        let clouds_depth_a = get("clouds_depth_a", TextureFormat::R32Float, clouds_size, none);
        let clouds_depth_b = get("clouds_depth_b", TextureFormat::R32Float, clouds_size, none);
        let (clouds, clouds_history, clouds_depth, clouds_depth_history) =
            if frame_count.0 % 2 == 0 {
                (clouds_a, clouds_b, clouds_depth_a, clouds_depth_b)
            } else {
                (clouds_b, clouds_a, clouds_depth_b, clouds_depth_a)
            };
        let clouds_trace = get(
            "clouds_trace",
            TextureFormat::Rgba16Float,
            trace_size,
            TextureUsages::empty(),
        );
        let clouds_trace_depth = get(
            "clouds_trace_depth",
            TextureFormat::R32Float,
            trace_size,
            TextureUsages::empty(),
        );

        commands.entity(entity).insert({
            AtmosphereTextures {
                transmittance_lut,
                multiscattering_lut,
                sky_view_lut,
                aerial_view_lut,
                cloud_shadow_map,
                cloud_shadow_map_tmp,
                cloud_shadow_map_history,
                clouds,
                clouds_depth,
                clouds_history,
                clouds_depth_history,
                clouds_trace,
                clouds_trace_depth,
            }
        });
    }
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
#[error("ScatteringMedium missing with id {0:?}: make sure the asset was not removed.")]
struct ScatteringMediumMissingError(AssetId<ScatteringMedium>);

/// The shader-uniform representation of an Atmosphere.
#[derive(Clone, Component, ShaderType)]
pub struct GpuAtmosphere {
    pub ground_albedo: Vec3,
    pub inner_radius: f32,
    pub outer_radius: f32,
    pub world_to_atmosphere: Mat4,
    pub atmosphere_to_world: Mat4,
}

pub fn prepare_atmosphere_uniforms(
    mut commands: Commands,
    atmospheres: Query<(Entity, &ExtractedAtmosphere)>,
) -> Result<(), BevyError> {
    for (entity, atmosphere) in atmospheres {
        commands.entity(entity).insert(GpuAtmosphere {
            ground_albedo: atmosphere.ground_albedo,
            inner_radius: atmosphere.inner_radius,
            outer_radius: atmosphere.outer_radius,
            world_to_atmosphere: atmosphere.world_to_atmosphere,
            atmosphere_to_world: atmosphere.atmosphere_to_world,
        });
    }
    Ok(())
}

#[derive(Resource, Default)]
pub struct AtmosphereTransforms {
    uniforms: DynamicUniformBuffer<AtmosphereTransform>,
}

impl AtmosphereTransforms {
    #[inline]
    pub fn uniforms(&self) -> &DynamicUniformBuffer<AtmosphereTransform> {
        &self.uniforms
    }
}

/// Transforms between world space and atmosphere space.
///
/// Up is the local planet surface normal, so the horizon stays along the x-z
/// plane for horizon-detail parameterization. Back is chosen from a constant
/// world-horizontal direction such as `Vec3A::NEG_Z`, then projected orthogonal
/// to up. It may drift slightly from world-horizontal but stays camera-independent.
#[derive(ShaderType)]
pub struct AtmosphereTransform {
    world_from_atmosphere: Mat4,
    atmosphere_from_world: Mat4,
}

#[derive(Component)]
pub struct AtmosphereTransformsOffset {
    index: u32,
}

impl AtmosphereTransformsOffset {
    #[inline]
    pub fn index(&self) -> u32 {
        self.index
    }
}

pub(super) fn prepare_atmosphere_transforms(
    views: Query<
        (Entity, &ExtractedView, &GpuAtmosphere),
        (With<ExtractedAtmosphere>, With<Camera3d>),
    >,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    mut atmo_uniforms: ResMut<AtmosphereTransforms>,
    mut commands: Commands,
) {
    let atmo_count = views.iter().len();
    let Some(mut writer) =
        atmo_uniforms
            .uniforms
            .get_writer(atmo_count, &render_device, &render_queue)
    else {
        return;
    };

    for (entity, view, gpu_atmosphere) in &views {
        // Camera position in atmosphere space
        let cam_world = view.world_from_view.translation();
        let cam_pos = Vec3A::from(
            gpu_atmosphere
                .world_to_atmosphere
                .transform_point3(cam_world),
        );

        // Up is the local planet surface normal.
        let atmo_y = cam_pos.try_normalize().unwrap_or(Vec3A::Y);

        // World-horizontal reference for back, projected orthogonal to atmo_y.
        let world_ref = Vec3A::NEG_Z;
        let ref_horizontal = world_ref - atmo_y * atmo_y.dot(world_ref);
        let atmo_z = ref_horizontal.try_normalize().unwrap_or_else(|| {
            // `NEG_Z` is degenerate at the poles of a Z-up world.
            let fallback_ref = Vec3A::NEG_Y;
            (fallback_ref - atmo_y * atmo_y.dot(fallback_ref)).normalize()
        });
        let atmo_x = atmo_y.cross(atmo_z).normalize();

        let world_from_atmosphere = Mat4::from(Affine3A::from_cols(
            atmo_x,
            atmo_y,
            atmo_z,
            view.world_from_view.translation_vec3a(),
        ));
        // The shader only uses the upper-left 3x3 block, where transpose equals inverse for
        // orthonormal matrices and is cheaper than computing the full inverse.
        let atmosphere_from_world = world_from_atmosphere.transpose();

        commands.entity(entity).insert(AtmosphereTransformsOffset {
            index: writer.write(&AtmosphereTransform {
                world_from_atmosphere,
                atmosphere_from_world,
            }),
        });
    }
}

#[derive(Component)]
pub(crate) struct AtmosphereBindGroups {
    pub transmittance_lut: BindGroup,
    pub multiscattering_lut: BindGroup,
    pub sky_view_lut: BindGroup,
    pub aerial_view_lut: BindGroup,
    pub cloud_shadow_map: Option<BindGroup>,
    // Accessed from the render-world graph; some tooling can miss the cross-module usage.
    #[allow(dead_code)]
    pub cloud_shadow_filter_a_to_b: Option<BindGroup>,
    // Accessed from the render-world graph; some tooling can miss the cross-module usage.
    #[allow(dead_code)]
    pub cloud_shadow_filter_b_to_a: Option<BindGroup>,
    /// Temporal filter bind group; present when cloud_shadow_temporal_enabled.
    pub cloud_shadow_temporal: Option<BindGroup>,
    pub render_sky_clouds: Option<BindGroup>,
    pub render_sky_no_clouds: BindGroup,
    /// `clouds_raymarch` pass (only for non-MSAA views with a `CloudLayer`).
    pub clouds_raymarch: Option<BindGroup>,
    /// `clouds_reconstruct` pass (same conditions as `clouds_raymarch`).
    pub clouds_reconstruct: Option<BindGroup>,
}

#[derive(Copy, Clone, Debug, thiserror::Error)]
enum AtmosphereBindGroupError {
    #[error("Failed to prepare atmosphere bind groups. Atmosphere uniform buffer missing")]
    Atmosphere,
    #[error(
        "Failed to prepare atmosphere bind groups. AtmosphereTransforms uniform buffer missing"
    )]
    Transforms,
    #[error("Failed to prepare atmosphere bind groups. AtmosphereSettings uniform buffer missing")]
    Settings,
    #[error("Failed to prepare atmosphere bind groups. View uniform buffer missing")]
    ViewUniforms,
    #[error("Failed to prepare atmosphere bind groups. Light uniform buffer missing")]
    LightUniforms,
}

pub(super) fn prepare_cloud_shadow_temporal_params(
    views: Query<
        (&ExtractedView, &ExtractedAtmosphere, &GpuAtmosphereSettings),
        (With<Camera3d>, With<ExtractedAtmosphere>),
    >,
    directional_lights: Query<&ExtractedDirectionalLight>,
    mut temporal_state: ResMut<CloudShadowTemporalState>,
    mut temporal_params_buffer: ResMut<CloudShadowTemporalParamsBuffer>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    frame_count: Res<FrameCount>,
) {
    let Some((view, atmosphere, settings)) = views.iter().next() else {
        return;
    };
    if !settings.cloud_shadow_uses_temporal_pass() {
        return;
    }
    let light_dir = directional_lights
        .iter()
        .next()
        .map(|light| Vec3::from(light.transform.back()))
        .unwrap_or(Vec3::NEG_Y);

    let extent = settings.cloud_shadow_map_extent;
    // When only time-slicing (no temporal filtering), traced texels replace history outright.
    let temporal_alpha = if settings.cloud_shadow_temporal_enabled != 0 {
        settings.cloud_shadow_temporal_alpha
    } else {
        1.0
    };
    let light_rotation_cut_deg = settings.cloud_shadow_temporal_light_rotation_cut_deg;

    let inner_radius = atmosphere.inner_radius;
    let camera_pos = view.world_from_view.translation();
    let world_pos = atmosphere.world_to_atmosphere.transform_point3(camera_pos);
    let min_radius = inner_radius + 1.0;
    let r = world_pos.length();
    let anchor = if r < min_radius {
        world_pos.normalize_or_zero() * min_radius
    } else {
        world_pos
    };

    let trace_dir = (-light_dir).normalize_or_zero();
    let world_up = Vec3::Y;
    let a = if trace_dir.dot(world_up).abs() > 0.999 {
        Vec3::X
    } else {
        world_up
    };
    let basis_x = a.cross(trace_dir).normalize_or_zero();
    let basis_y = trace_dir.cross(basis_x);

    let size = settings.cloud_shadow_map_size;
    let res = size.as_vec2();
    let texel_size = (2.0 * extent) / res.max(Vec2::ONE);
    let ax = anchor.dot(basis_x);
    let ay = anchor.dot(basis_y);
    let snapped_ax = (ax / texel_size.x).round() * texel_size.x;
    let snapped_ay = (ay / texel_size.y).round() * texel_size.y;
    let anchor = anchor + basis_x * (snapped_ax - ax) + basis_y * (snapped_ay - ay);

    let prev_anchor = temporal_state.anchor;
    let prev_light_dir = temporal_state.light_dir;
    let anchor_moved = (anchor - prev_anchor).length_squared() > 1e-6;
    let light_rot_cos = (light_rotation_cut_deg * std::f32::consts::PI / 180.0).cos();
    let light_rotated = prev_light_dir.dot(light_dir.normalize_or_zero()) < light_rot_cos;
    let history_valid = temporal_state.initialized && !light_rotated;

    *temporal_state = CloudShadowTemporalState {
        anchor,
        light_dir: light_dir.normalize_or_zero(),
        basis_x,
        basis_y,
        initialized: true,
    };

    let prev_anchor = if history_valid { prev_anchor } else { anchor };
    let prev_light_dir_to_light = if history_valid {
        prev_light_dir
    } else {
        light_dir.normalize_or_zero()
    };
    let prev_trace_dir = -prev_light_dir_to_light;
    // Build the previous basis from the trace direction exactly like the current one (and like
    // `build_light_basis` in the shaders); building it from the direction *to* the light mirrors
    // the X axis and reprojects the history to the wrong texels.
    let prev_a = if prev_trace_dir.dot(Vec3::Y).abs() > 0.999 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let prev_basis_x = prev_a.cross(prev_trace_dir).normalize_or_zero();
    let prev_basis_y = prev_trace_dir.cross(prev_basis_x);

    temporal_params_buffer
        .buffer
        .set(CloudShadowTemporalParams {
            curr_anchor: anchor,
            curr_light_dir: trace_dir,
            curr_basis_x: basis_x,
            curr_basis_y: basis_y,
            prev_anchor,
            prev_light_dir: prev_trace_dir,
            prev_basis_x,
            prev_basis_y,
            extent,
            temporal_alpha,
            history_valid: history_valid as u32,
            anchor_moved: anchor_moved as u32,
            size,
            interleave: settings.cloud_shadow_map_update_interleave.max(1),
            frame_index: frame_count.0,
        });
    temporal_params_buffer
        .buffer
        .write_buffer(&render_device, &render_queue);
}

pub(super) fn prepare_atmosphere_bind_groups(
    views: Query<
        (
            Entity,
            &ExtractedAtmosphere,
            &AtmosphereTextures,
            &ViewDepthStencilTexture,
            &ViewShadowBindings,
            &Msaa,
            Option<&CloudLayer>,
            Option<&GpuAtmosphereSettings>,
            Option<&CloudsTemporalParamsUniform>,
        ),
        (With<Camera3d>, With<ExtractedAtmosphere>),
    >,
    render_device: Res<RenderDevice>,
    layouts: Res<AtmosphereBindGroupLayouts>,
    render_sky_layouts: Res<RenderSkyBindGroupLayouts>,
    (atmosphere_sampler, cloud_noise_sampler, shadow_samplers): (
        Res<AtmosphereSampler>,
        Res<CloudNoiseSampler>,
        Res<ShadowSamplers>,
    ),
    (bluenoise, render_images, fallback_image): (
        Res<Bluenoise>,
        Res<RenderAssets<GpuImage>>,
        Res<FallbackImage>,
    ),
    view_uniforms: Res<ViewUniforms>,
    lights_uniforms: Res<LightMeta>,
    atmosphere_transforms: Res<AtmosphereTransforms>,
    atmosphere_uniforms: Res<ComponentUniforms<GpuAtmosphere>>,
    settings_uniforms: Res<ComponentUniforms<GpuAtmosphereSettings>>,
    gpu_media: Res<RenderAssets<GpuScatteringMedium>>,
    medium_sampler: Res<ScatteringMediumSampler>,
    pipeline_cache: Res<PipelineCache>,
    (cloud_layer_uniforms, fbm_noise_texture, perlin_worley_noise_texture, detail_noise_texture, curl_noise_texture, temporal_params): (
        Res<ComponentUniforms<CloudLayer>>,
        Res<FbmNoiseTexture>,
        Res<PerlinWorleyNoiseTexture>,
        Res<DetailNoiseTexture>,
        Res<CurlNoiseTexture>,
        Res<CloudShadowTemporalParamsBuffer>,
    ),
    mut commands: Commands,
) -> Result<(), BevyError> {
    if views.iter().len() == 0 {
        return Ok(());
    }

    let atmosphere_binding = atmosphere_uniforms
        .binding()
        .ok_or(AtmosphereBindGroupError::Atmosphere)?;

    let transforms_binding = atmosphere_transforms
        .uniforms()
        .binding()
        .ok_or(AtmosphereBindGroupError::Transforms)?;

    let settings_binding = settings_uniforms
        .binding()
        .ok_or(AtmosphereBindGroupError::Settings)?;

    let view_binding = view_uniforms
        .uniforms
        .binding()
        .ok_or(AtmosphereBindGroupError::ViewUniforms)?;

    let lights_binding = lights_uniforms
        .view_gpu_lights
        .binding()
        .ok_or(AtmosphereBindGroupError::LightUniforms)?;

    // Optional: when no view has a CloudLayer component, the uniform buffer can be absent.
    // We build no-cloud bind groups in that case.
    let cloud_layer_binding = cloud_layer_uniforms.binding();

    // STBN texture for temporal stratification; use fallback when not yet loaded.
    let stbn_view: std::borrow::Cow<'_, TextureView> = match render_images.get(&bluenoise.texture) {
        Some(gpu) => std::borrow::Cow::Owned(gpu.texture.create_view(&TextureViewDescriptor {
            dimension: Some(TextureViewDimension::D2Array),
            ..Default::default()
        })),
        None => std::borrow::Cow::Borrowed(&fallback_image.d2_array.texture_view),
    };

    for (
        entity,
        atmosphere,
        textures,
        view_depth_texture,
        shadow_bindings,
        msaa,
        cloud_layer,
        settings,
        temporal_uniform,
    ) in &views
    {
        let Some(depth_view) = view_depth_texture
            .attachment
            .depth_stencil_views()
            .depth_only_view()
        else {
            continue;
        };
        let temporal_enabled = settings
            .map(GpuAtmosphereSettings::cloud_shadow_uses_temporal_pass)
            .unwrap_or(false);
        let gpu_medium = gpu_media
            .get(atmosphere.medium)
            .ok_or(ScatteringMediumMissingError(atmosphere.medium))?;
        let transmittance_lut = render_device.create_bind_group(
            "transmittance_lut_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.transmittance_lut),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                // scattering medium luts and sampler
                (5, &gpu_medium.density_lut_view),
                (6, &gpu_medium.scattering_lut_view),
                (7, medium_sampler.sampler()),
                // transmittance lut storage texture
                (13, &textures.transmittance_lut.default_view),
            )),
        );

        let multiscattering_lut = render_device.create_bind_group(
            "multiscattering_lut_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.multiscattering_lut),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                // scattering medium luts and sampler
                (5, &gpu_medium.density_lut_view),
                (6, &gpu_medium.scattering_lut_view),
                (7, medium_sampler.sampler()),
                // atmosphere luts and sampler
                (8, &textures.transmittance_lut.default_view),
                (12, &**atmosphere_sampler),
                // multiscattering lut storage texture
                (13, &textures.multiscattering_lut.default_view),
            )),
        );

        let sky_view_lut = render_device.create_bind_group(
            "sky_view_lut_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.sky_view_lut),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                (2, transforms_binding.clone()),
                (3, view_binding.clone()),
                (4, lights_binding.clone()),
                // scattering medium luts and sampler
                (5, &gpu_medium.density_lut_view),
                (6, &gpu_medium.scattering_lut_view),
                (7, medium_sampler.sampler()),
                // atmosphere luts and sampler
                (8, &textures.transmittance_lut.default_view),
                (9, &textures.multiscattering_lut.default_view),
                (12, &**atmosphere_sampler),
                // sky view lut storage texture
                (13, &textures.sky_view_lut.default_view),
                // shadow maps
                (14, &shadow_bindings.point_light_depth_texture_view),
                (15, &shadow_samplers.point_light_comparison_sampler),
                (16, &shadow_bindings.directional_light_depth_texture_view),
                (17, &shadow_samplers.directional_light_comparison_sampler),
            )),
        );

        let aerial_view_lut = render_device.create_bind_group(
            "aerial_view_lut_bind_group",
            &pipeline_cache.get_bind_group_layout(&layouts.aerial_view_lut),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                (3, view_binding.clone()),
                (4, lights_binding.clone()),
                // scattering medium luts and sampler
                (5, &gpu_medium.density_lut_view),
                (6, &gpu_medium.scattering_lut_view),
                (7, medium_sampler.sampler()),
                // atmosphere luts and sampler
                (8, &textures.transmittance_lut.default_view),
                (9, &textures.multiscattering_lut.default_view),
                (12, &**atmosphere_sampler),
                // aerial view lut storage texture
                (13, &textures.aerial_view_lut.default_view),
                // shadow maps
                (14, &shadow_bindings.point_light_depth_texture_view),
                (15, &shadow_samplers.point_light_comparison_sampler),
                (16, &shadow_bindings.directional_light_depth_texture_view),
                (17, &shadow_samplers.directional_light_comparison_sampler),
            )),
        );

        // When temporal enabled: trace writes to tmp; filter reads tmp first.
        // When temporal disabled: trace writes to cloud_shadow_map; filter reads cloud_shadow_map first.
        let trace_storage = if temporal_enabled {
            &textures.cloud_shadow_map_tmp.default_view
        } else {
            &textures.cloud_shadow_map.default_view
        };
        let (filter_read_a, filter_write_a) = if temporal_enabled {
            (
                &textures.cloud_shadow_map_tmp.default_view,
                &textures.cloud_shadow_map.default_view,
            )
        } else {
            (
                &textures.cloud_shadow_map.default_view,
                &textures.cloud_shadow_map_tmp.default_view,
            )
        };
        let (filter_read_b, filter_write_b) = if temporal_enabled {
            (
                &textures.cloud_shadow_map.default_view,
                &textures.cloud_shadow_map_tmp.default_view,
            )
        } else {
            (
                &textures.cloud_shadow_map_tmp.default_view,
                &textures.cloud_shadow_map.default_view,
            )
        };

        let cloud_shadow_map = cloud_layer_binding.as_ref().map(|cloud_layer_binding| {
            render_device.create_bind_group(
                "cloud_shadow_map_bind_group",
                &pipeline_cache.get_bind_group_layout(&layouts.cloud_shadow_map),
                &BindGroupEntries::with_indices((
                    (0, atmosphere_binding.clone()),
                    (1, settings_binding.clone()),
                    (3, view_binding.clone()),
                    (4, lights_binding.clone()),
                    (20, cloud_layer_binding.clone()),
                    (21, &fbm_noise_texture.texture.default_view),
                    (22, &**cloud_noise_sampler),
                    (24, &perlin_worley_noise_texture.texture.default_view),
                    (25, stbn_view.as_ref()),
                    (33, &detail_noise_texture.texture.default_view),
                    (34, &curl_noise_texture.texture.default_view),
                    (13, trace_storage),
                )),
            )
        });

        // Cloud shadow spatial filter ping-pong bind groups.
        // When temporal: A=tmp (trace output), B=cloud_shadow_map. When not: A=cloud_shadow_map, B=tmp.
        let cloud_shadow_filter_a_to_b = cloud_shadow_map.as_ref().map(|_| {
            render_device.create_bind_group(
                "cloud_shadow_filter_a_to_b_bind_group",
                &pipeline_cache.get_bind_group_layout(&layouts.cloud_shadow_filter),
                &BindGroupEntries::with_indices((
                    (1, settings_binding.clone()),
                    (12, &**atmosphere_sampler),
                    (23, filter_read_a),
                    (13, filter_write_a),
                )),
            )
        });

        let cloud_shadow_filter_b_to_a = cloud_shadow_map.as_ref().map(|_| {
            render_device.create_bind_group(
                "cloud_shadow_filter_b_to_a_bind_group",
                &pipeline_cache.get_bind_group_layout(&layouts.cloud_shadow_filter),
                &BindGroupEntries::with_indices((
                    (1, settings_binding.clone()),
                    (12, &**atmosphere_sampler),
                    (23, filter_read_b),
                    (13, filter_write_b),
                )),
            )
        });

        // Temporal filter: curr=tmp (or cloud_shadow_map when no filter), prev=history, out=cloud_shadow_map.
        let cloud_shadow_temporal = (cloud_shadow_map.is_some() && temporal_enabled)
            .then(|| {
                temporal_params
                    .buffer
                    .binding()
                    .map(|temporal_params_binding| {
                        render_device.create_bind_group(
                            "cloud_shadow_temporal_bind_group",
                            &pipeline_cache.get_bind_group_layout(&layouts.cloud_shadow_temporal),
                            &BindGroupEntries::with_indices((
                                (0, temporal_params_binding),
                                (1, &textures.cloud_shadow_map_tmp.default_view),
                                (2, &textures.cloud_shadow_map_history.default_view),
                                (3, &**atmosphere_sampler),
                                (4, &textures.cloud_shadow_map.default_view),
                            )),
                        )
                    })
            })
            .flatten();

        let render_sky_no_clouds = render_device.create_bind_group(
            "render_sky_bind_group_no_clouds",
            &pipeline_cache.get_bind_group_layout(if *msaa == Msaa::Off {
                &render_sky_layouts.render_sky_no_clouds
            } else {
                &render_sky_layouts.render_sky_msaa_no_clouds
            }),
            &BindGroupEntries::with_indices((
                // uniforms
                (0, atmosphere_binding.clone()),
                (1, settings_binding.clone()),
                (2, transforms_binding.clone()),
                (3, view_binding.clone()),
                (4, lights_binding.clone()),
                // scattering medium luts and sampler
                (5, &gpu_medium.density_lut_view),
                (6, &gpu_medium.scattering_lut_view),
                (7, medium_sampler.sampler()),
                // atmosphere luts and sampler
                (8, &textures.transmittance_lut.default_view),
                (9, &textures.multiscattering_lut.default_view),
                (10, &textures.sky_view_lut.default_view),
                (11, &textures.aerial_view_lut.default_view),
                (12, &**atmosphere_sampler),
                // view depth texture
                (13, depth_view),
                // shadow maps
                (14, &shadow_bindings.point_light_depth_texture_view),
                (15, &shadow_samplers.point_light_comparison_sampler),
                (16, &shadow_bindings.directional_light_depth_texture_view),
                (17, &shadow_samplers.directional_light_comparison_sampler),
            )),
        );

        // Only create clouds bind group for views that have CloudLayer; otherwise use no_clouds
        // to avoid passing clouds layout to no_clouds pipeline when disabling clouds.
        let render_sky_clouds = cloud_layer_binding
            .as_ref()
            .filter(|_| cloud_layer.is_some())
            .map(|cloud_layer_binding| {
                render_device.create_bind_group(
                    "render_sky_bind_group_clouds",
                    &pipeline_cache.get_bind_group_layout(if *msaa == Msaa::Off {
                        &render_sky_layouts.render_sky_clouds
                    } else {
                        &render_sky_layouts.render_sky_msaa_clouds
                    }),
                    &BindGroupEntries::with_indices((
                        // uniforms
                        (0, atmosphere_binding.clone()),
                        (1, settings_binding.clone()),
                        (2, transforms_binding.clone()),
                        (3, view_binding.clone()),
                        (4, lights_binding.clone()),
                        // scattering medium luts and sampler
                        (5, &gpu_medium.density_lut_view),
                        (6, &gpu_medium.scattering_lut_view),
                        (7, medium_sampler.sampler()),
                        // atmosphere luts and sampler
                        (8, &textures.transmittance_lut.default_view),
                        (9, &textures.multiscattering_lut.default_view),
                        (10, &textures.sky_view_lut.default_view),
                        (11, &textures.aerial_view_lut.default_view),
                        (12, &**atmosphere_sampler),
                        // view depth texture
                        (13, depth_view),
                        // shadow maps
                        (14, &shadow_bindings.point_light_depth_texture_view),
                        (15, &shadow_samplers.point_light_comparison_sampler),
                        (16, &shadow_bindings.directional_light_depth_texture_view),
                        (17, &shadow_samplers.directional_light_comparison_sampler),
                        // clouds
                        (20, cloud_layer_binding.clone()),
                        (21, &fbm_noise_texture.texture.default_view),
                        (22, &**cloud_noise_sampler),
                        (23, &textures.cloud_shadow_map.default_view),
                        (24, &perlin_worley_noise_texture.texture.default_view),
                        (25, stbn_view.as_ref()),
                        (33, &detail_noise_texture.texture.default_view),
                        (34, &curl_noise_texture.texture.default_view),
                        // clouds_raymarch outputs
                        (26, &textures.clouds.default_view),
                        (27, &textures.clouds_depth.default_view),
                    )),
                )
            });

        let clouds_raymarch = cloud_layer_binding
            .as_ref()
            .filter(|_| cloud_layer.is_some() && *msaa == Msaa::Off)
            .map(|cloud_layer_binding| {
                render_device.create_bind_group(
                    "clouds_raymarch_bind_group",
                    &pipeline_cache.get_bind_group_layout(&layouts.clouds_raymarch),
                    &BindGroupEntries::with_indices((
                        (0, atmosphere_binding.clone()),
                        (1, settings_binding.clone()),
                        (2, transforms_binding.clone()),
                        (3, view_binding.clone()),
                        (4, lights_binding.clone()),
                        (5, &gpu_medium.density_lut_view),
                        (6, &gpu_medium.scattering_lut_view),
                        (7, medium_sampler.sampler()),
                        (8, &textures.transmittance_lut.default_view),
                        (9, &textures.multiscattering_lut.default_view),
                        (10, &textures.sky_view_lut.default_view),
                        (11, &textures.aerial_view_lut.default_view),
                        (12, &**atmosphere_sampler),
                        (13, depth_view),
                        (20, cloud_layer_binding.clone()),
                        (21, &fbm_noise_texture.texture.default_view),
                        (22, &**cloud_noise_sampler),
                        (23, &textures.cloud_shadow_map.default_view),
                        (24, &perlin_worley_noise_texture.texture.default_view),
                        (25, stbn_view.as_ref()),
                        (33, &detail_noise_texture.texture.default_view),
                        (34, &curl_noise_texture.texture.default_view),
                        (26, &textures.clouds_trace.default_view),
                        (27, &textures.clouds_trace_depth.default_view),
                    )),
                )
            });

        let clouds_reconstruct = clouds_raymarch
            .as_ref()
            .zip(temporal_uniform.and_then(|u| u.0.binding()))
            .map(|(_, params_binding)| {
                render_device.create_bind_group(
                    "clouds_reconstruct_bind_group",
                    &pipeline_cache.get_bind_group_layout(&layouts.clouds_reconstruct),
                    &BindGroupEntries::with_indices((
                        (0, atmosphere_binding.clone()),
                        (3, view_binding.clone()),
                        (12, &**atmosphere_sampler),
                        (26, &textures.clouds_trace.default_view),
                        (27, &textures.clouds_trace_depth.default_view),
                        (28, &textures.clouds_history.default_view),
                        (29, &textures.clouds_depth_history.default_view),
                        (30, params_binding),
                        (31, &textures.clouds.default_view),
                        (32, &textures.clouds_depth.default_view),
                    )),
                )
            });

        commands.entity(entity).insert(AtmosphereBindGroups {
            transmittance_lut,
            multiscattering_lut,
            sky_view_lut,
            aerial_view_lut,
            cloud_shadow_map,
            cloud_shadow_filter_a_to_b,
            cloud_shadow_filter_b_to_a,
            cloud_shadow_temporal,
            render_sky_clouds,
            render_sky_no_clouds,
            clouds_raymarch,
            clouds_reconstruct,
        });
    }

    Ok(())
}

#[derive(ShaderType)]
#[repr(C)]
pub(crate) struct AtmosphereData {
    pub atmosphere: GpuAtmosphere,
    pub settings: GpuAtmosphereSettings,
}

#[derive(Component)]
pub struct AtmosphereBuffer {
    pub(crate) buffer: StorageBuffer<AtmosphereData>,
}

#[derive(Resource)]
pub struct CloudShadowTemporalParamsBuffer {
    pub(crate) buffer: UniformBuffer<CloudShadowTemporalParams>,
}

impl Default for CloudShadowTemporalParamsBuffer {
    fn default() -> Self {
        Self {
            buffer: UniformBuffer::default(),
        }
    }
}

pub(crate) fn prepare_atmosphere_buffers(
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut views: Query<
        (
            Entity,
            &GpuAtmosphere,
            &GpuAtmosphereSettings,
            Option<&mut AtmosphereBuffer>,
            bevy_ecs::query::Has<CloudLayer>,
        ),
        With<ExtractedAtmosphere>,
    >,
    mut commands: Commands,
) {
    for (entity, atmosphere, settings, existing_buffer, has_clouds) in &mut views {
        let mut settings = settings.clone();
        if !has_clouds {
            // PBR shading skips the cloud shadow map lookup when the extent is zero, so a
            // stale map from a removed `CloudLayer` never darkens surfaces.
            settings.cloud_shadow_map_extent = 0.0;
        }
        let data = AtmosphereData {
            atmosphere: atmosphere.clone(),
            settings,
        };
        if let Some(mut atmosphere_buffer) = existing_buffer {
            atmosphere_buffer.buffer.set(data);
            atmosphere_buffer.buffer.write_buffer(&device, &queue);
        } else {
            let mut buffer = StorageBuffer::from(data);
            buffer.write_buffer(&device, &queue);
            commands.entity(entity).insert(AtmosphereBuffer { buffer });
        }
    }
}
