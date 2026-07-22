use super::SolariLighting;
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy_anti_alias::dlss::{
    Dlss, DlssRayReconstructionFeature, ViewDlssRayReconstructionTextures,
};
use bevy_camera::MainPassResolutionOverride;
use bevy_diagnostic::FrameCount;
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy_ecs::query::Has;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    system::{Commands, Query, Res},
};
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy_image::ToExtents;
use bevy_math::UVec2;
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::{Buffer, BufferDescriptor, BufferInitDescriptor, BufferUsages},
    renderer::{RenderDevice, RenderQueue},
};
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy_render::{
    render_resource::{
        TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureViewDescriptor,
    },
    texture::CachedTexture,
};
use bytemuck::{Pod, Zeroable};

/// Size of the `LightSample` shader struct in bytes.
const LIGHT_SAMPLE_STRUCT_SIZE: u64 = 8;

/// Size of the `ResolvedLightSamplePacked` shader struct in bytes.
const RESOLVED_LIGHT_SAMPLE_STRUCT_SIZE: u64 = 24;

/// Size of the `Reservoir` shader struct in bytes.
const RESERVOIR_STRUCT_SIZE: u64 = 48;

pub const LIGHT_TILE_BLOCKS: u64 = 128;
pub const LIGHT_TILE_SAMPLES_PER_BLOCK: u64 = 1024;

/// Amount of entries in the world cache (must be a power of 2, and >= 2^10)
pub const WORLD_CACHE_SIZE: u64 = 2u64.pow(20);

/// Layout constants for the packed `WorldCache` buffer.
///
/// The `ShaderType` mirror lives in this module and is intentionally private so it cannot be
/// constructed, because it would be too large.
mod world_cache_layout {
    use bevy_math::{Vec3, Vec4};
    use bevy_render::render_resource::ShaderType;

    const WORLD_CACHE_LEN: usize = super::WORLD_CACHE_SIZE as usize;

    #[derive(ShaderType)]
    struct GeometryData {
        world_position: Vec3,
        padding_a: u32,
        world_normal: Vec3,
        padding_b: u32,
    }

    #[derive(ShaderType)]
    struct WorldCache {
        checksums: [u32; WORLD_CACHE_LEN],
        life: [u32; WORLD_CACHE_LEN],
        radiance: [Vec4; WORLD_CACHE_LEN],
        geometry_data: [GeometryData; WORLD_CACHE_LEN],
        luminance_deltas: [f32; WORLD_CACHE_LEN],
        active_cells_new_radiance: [Vec3; WORLD_CACHE_LEN],
        a: [u32; WORLD_CACHE_LEN],
        b: [u32; WORLD_CACHE_LEN / 1024],
        active_cell_indices: [u32; WORLD_CACHE_LEN],
        active_cells_count: u32,
    }

    // `ShaderType::METADATA` is internal, but we need the field offset and size without
    // constructing this large layout type.
    pub const ACTIVE_CELLS_COUNT_OFFSET: u64 = WorldCache::METADATA.last_offset();
    /// Must stay under wgpu's default `max_storage_buffer_binding_size` (128 MiB or 2^27 bytes).
    pub const BUFFER_SIZE: u64 = WorldCache::METADATA.min_size().get();
}

pub const WORLD_CACHE_ACTIVE_CELLS_COUNT_OFFSET: u64 =
    world_cache_layout::ACTIVE_CELLS_COUNT_OFFSET;

/// GPU representation of the user-configurable [`SolariLighting`] settings, plus
/// per-frame state.
///
/// Field order and types must match the `SolariLightingSettings` struct in
/// `realtime_bindings.wgsl`.
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct SolariLightingUniforms {
    confidence_weight_cap: f32,
    primary_di_samples: u32,
    secondary_di_samples: u32,
    max_bounces: u32,
    world_cache_max_temporal_samples: f32,
    world_cache_direct_light_sample_count: u32,
    world_cache_max_gi_ray_distance: f32,
    world_cache_cell_updates_soft_target: u32,
    world_cache_position_base_cell_size: f32,
    world_cache_position_lod_scale: f32,
    frame_rng: u32,
    reset: u32,
}

impl SolariLightingUniforms {
    fn new(settings: &SolariLighting, frame_count: u32) -> Self {
        Self {
            confidence_weight_cap: settings.confidence_weight_cap,
            primary_di_samples: settings.primary_di_samples,
            secondary_di_samples: settings.secondary_di_samples,
            max_bounces: settings.max_bounces,
            world_cache_max_temporal_samples: settings.world_cache_max_temporal_samples,
            world_cache_direct_light_sample_count: settings.world_cache_direct_light_sample_count,
            world_cache_max_gi_ray_distance: settings.world_cache_max_gi_ray_distance,
            world_cache_cell_updates_soft_target: settings.world_cache_cell_updates_soft_target,
            world_cache_position_base_cell_size: settings.world_cache_position_base_cell_size,
            world_cache_position_lod_scale: settings.world_cache_position_lod_scale,
            frame_rng: frame_count.wrapping_mul(5782582),
            reset: settings.reset as u32,
        }
    }
}

/// Internal rendering resources used for Solari lighting.
#[derive(Component)]
pub struct SolariLightingResources {
    pub constants: Buffer,
    pub light_tile_samples: Buffer,
    pub light_tile_resolved_samples: Buffer,
    pub reservoirs_a: Buffer,
    pub reservoirs_b: Buffer,
    pub world_cache: Buffer,
    pub world_cache_active_cells_dispatch: Buffer,
    pub view_size: UVec2,
}

pub fn prepare_solari_lighting_resources(
    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))] query: Query<(
        Entity,
        &ExtractedCamera,
        &SolariLighting,
        Option<&SolariLightingResources>,
        Option<&MainPassResolutionOverride>,
    )>,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] query: Query<(
        Entity,
        &ExtractedCamera,
        &SolariLighting,
        Option<&SolariLightingResources>,
        Option<&MainPassResolutionOverride>,
        Has<Dlss<DlssRayReconstructionFeature>>,
    )>,
    render_device: Res<RenderDevice>,
    render_queue: Res<RenderQueue>,
    frame_count: Res<FrameCount>,
    mut commands: Commands,
) {
    for query_item in &query {
        #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
        let (entity, camera, solari_lighting, solari_lighting_resources, resolution_override) =
            query_item;
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        let (
            entity,
            camera,
            solari_lighting,
            solari_lighting_resources,
            resolution_override,
            has_dlss_rr,
        ) = query_item;

        let Some(mut view_size) = camera.physical_viewport_size else {
            continue;
        };
        if let Some(MainPassResolutionOverride(resolution_override)) = resolution_override {
            view_size = *resolution_override;
        }

        let uniforms = SolariLightingUniforms::new(solari_lighting, frame_count.0);

        if let Some(solari_lighting_resources) = solari_lighting_resources
            && solari_lighting_resources.view_size == view_size
        {
            // The constants uniform can change every frame, so always upload it.
            render_queue.write_buffer(
                &solari_lighting_resources.constants,
                0,
                bytemuck::bytes_of(&uniforms),
            );
            continue;
        }

        let constants = render_device.create_buffer_with_data(&BufferInitDescriptor {
            label: Some("solari_lighting_constants"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: BufferUsages::UNIFORM | BufferUsages::COPY_DST,
        });

        let light_tile_samples = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_light_tile_samples"),
            size: LIGHT_TILE_BLOCKS * LIGHT_TILE_SAMPLES_PER_BLOCK * LIGHT_SAMPLE_STRUCT_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let light_tile_resolved_samples = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_light_tile_resolved_samples"),
            size: LIGHT_TILE_BLOCKS
                * LIGHT_TILE_SAMPLES_PER_BLOCK
                * RESOLVED_LIGHT_SAMPLE_STRUCT_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let reservoirs_buffer = |name| {
            render_device.create_buffer(&BufferDescriptor {
                label: Some(name),
                size: (view_size.x * view_size.y) as u64 * RESERVOIR_STRUCT_SIZE,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };
        let reservoirs_a = reservoirs_buffer("solari_lighting_reservoirs_a");
        let reservoirs_b = reservoirs_buffer("solari_lighting_reservoirs_b");

        let world_cache = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache"),
            size: world_cache_layout::BUFFER_SIZE,
            usage: BufferUsages::STORAGE | BufferUsages::COPY_SRC,
            mapped_at_creation: false,
        });

        let world_cache_active_cells_dispatch = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_active_cells_dispatch"),
            size: size_of::<[u32; 3]>() as u64,
            usage: BufferUsages::INDIRECT | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        commands.entity(entity).insert(SolariLightingResources {
            constants,
            light_tile_samples,
            light_tile_resolved_samples,
            reservoirs_a,
            reservoirs_b,
            world_cache,
            world_cache_active_cells_dispatch,
            view_size,
        });

        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        if has_dlss_rr {
            let diffuse_albedo = render_device.create_texture(&TextureDescriptor {
                label: Some("solari_lighting_diffuse_albedo"),
                size: view_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            });
            let diffuse_albedo_view = diffuse_albedo.create_view(&TextureViewDescriptor::default());

            let specular_albedo = render_device.create_texture(&TextureDescriptor {
                label: Some("solari_lighting_specular_albedo"),
                size: view_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba8Unorm,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            });
            let specular_albedo_view =
                specular_albedo.create_view(&TextureViewDescriptor::default());

            let normal_roughness = render_device.create_texture(&TextureDescriptor {
                label: Some("solari_lighting_normal_roughness"),
                size: view_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rgba16Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            });
            let normal_roughness_view =
                normal_roughness.create_view(&TextureViewDescriptor::default());

            let specular_motion_vectors = render_device.create_texture(&TextureDescriptor {
                label: Some("solari_lighting_specular_motion_vectors"),
                size: view_size.to_extents(),
                mip_level_count: 1,
                sample_count: 1,
                dimension: TextureDimension::D2,
                format: TextureFormat::Rg16Float,
                usage: TextureUsages::TEXTURE_BINDING | TextureUsages::STORAGE_BINDING,
                view_formats: &[],
            });
            let specular_motion_vectors_view =
                specular_motion_vectors.create_view(&TextureViewDescriptor::default());

            commands
                .entity(entity)
                .insert(ViewDlssRayReconstructionTextures {
                    diffuse_albedo: CachedTexture {
                        texture: diffuse_albedo,
                        default_view: diffuse_albedo_view,
                    },
                    specular_albedo: CachedTexture {
                        texture: specular_albedo,
                        default_view: specular_albedo_view,
                    },
                    normal_roughness: CachedTexture {
                        texture: normal_roughness,
                        default_view: normal_roughness_view,
                    },
                    specular_motion_vectors: CachedTexture {
                        texture: specular_motion_vectors,
                        default_view: specular_motion_vectors_view,
                    },
                });
        }
    }
}
