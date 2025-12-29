use super::SolariLighting;
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy_anti_alias::dlss::{
    Dlss, DlssRayReconstructionFeature, ViewDlssRayReconstructionTextures,
};
use bevy_camera::MainPassResolutionOverride;
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy_ecs::query::Has;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::With,
    system::{Commands, Query, Res},
};
use bevy_math::UVec2;
use bevy_render::{
    camera::ExtractedCamera,
    render_resource::{Buffer, BufferDescriptor, BufferUsages},
    renderer::RenderDevice,
};
#[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
use bevy_render::{render_resource::TextureFormat, texture::CachedTexture};

/// Size of the GI `Reservoir` shader struct in bytes.
const GI_RESERVOIR_STRUCT_SIZE: u64 = 48;
/// Number of lights stored per world cache cell.
const WORLD_CACHE_CELL_LIGHT_COUNT: u64 = 8;

/// Size of the `LightSamplePacked` shader struct in bytes.
const PACKED_LIGHT_SAMPLE_STRUCT_SIZE: u64 = 28;
pub const LIGHT_TILE_BLOCKS: u64 = 128;
pub const LIGHT_TILE_SAMPLES_PER_BLOCK: u64 = 1024;

/// Amount of entries in the world cache (must be a power of 2, and >= 2^10)
pub const WORLD_CACHE_SIZE: u64 = 2u64.pow(20);

/// Internal rendering resources used for Solari lighting.
#[derive(Component)]
pub struct SolariLightingResources {
    pub light_tile_samples: Buffer,
    pub gi_reservoirs_a: Buffer,
    pub gi_reservoirs_b: Buffer,
    pub world_cache_checksums: Buffer,
    pub world_cache_life: Buffer,
    pub world_cache_radiance: Buffer,
    pub world_cache_geometry_data: Buffer,
    pub world_cache_light_data: Buffer,
    pub world_cache_light_data_new_lights: Buffer,
    pub world_cache_luminance_deltas: Buffer,
    pub world_cache_active_cells_new_radiance: Buffer,
    pub world_cache_a: Buffer,
    pub world_cache_b: Buffer,
    pub world_cache_active_cell_indices: Buffer,
    pub world_cache_active_cells_count: Buffer,
    pub world_cache_active_cells_dispatch: Buffer,
    pub view_size: UVec2,
}

pub fn prepare_solari_lighting_resources(
    #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))] query: Query<
        (
            Entity,
            &ExtractedCamera,
            Option<&SolariLightingResources>,
            Option<&MainPassResolutionOverride>,
        ),
        With<SolariLighting>,
    >,
    #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))] query: Query<
        (
            Entity,
            &ExtractedCamera,
            Option<&SolariLightingResources>,
            Option<&MainPassResolutionOverride>,
            Has<Dlss<DlssRayReconstructionFeature>>,
        ),
        With<SolariLighting>,
    >,
    render_device: Res<RenderDevice>,
    mut commands: Commands,
) {
    for query_item in &query {
        #[cfg(any(not(feature = "dlss"), feature = "force_disable_dlss"))]
        let (entity, camera, solari_lighting_resources, resolution_override) = query_item;
        #[cfg(all(feature = "dlss", not(feature = "force_disable_dlss")))]
        let (entity, camera, solari_lighting_resources, resolution_override, has_dlss_rr) =
            query_item;

        let Some(mut view_size) = camera.physical_viewport_size else {
            continue;
        };
        if let Some(MainPassResolutionOverride(resolution_override)) = resolution_override {
            view_size = *resolution_override;
        }

        if solari_lighting_resources.map(|r| r.view_size) == Some(view_size) {
            continue;
        }

        let light_tile_samples = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_light_tile_samples"),
            size: LIGHT_TILE_BLOCKS
                * LIGHT_TILE_SAMPLES_PER_BLOCK
                * PACKED_LIGHT_SAMPLE_STRUCT_SIZE,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let gi_reservoirs = |name| {
            render_device.create_buffer(&BufferDescriptor {
                label: Some(name),
                size: (view_size.x * view_size.y) as u64 * GI_RESERVOIR_STRUCT_SIZE,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            })
        };
        let gi_reservoirs_a = gi_reservoirs("solari_lighting_gi_reservoirs_a");
        let gi_reservoirs_b = gi_reservoirs("solari_lighting_gi_reservoirs_b");

        let world_cache_checksums = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_checksums"),
            size: WORLD_CACHE_SIZE * size_of::<u32>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let world_cache_life = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_life"),
            size: WORLD_CACHE_SIZE * size_of::<u32>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let world_cache_radiance = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_radiance"),
            size: WORLD_CACHE_SIZE * size_of::<[f32; 4]>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let world_cache_geometry_data = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_geometry_data"),
            size: WORLD_CACHE_SIZE * size_of::<[f32; 8]>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let world_cache_light_data = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_light_data"),
            size: WORLD_CACHE_SIZE
                * size_of::<[u64; 1 + WORLD_CACHE_CELL_LIGHT_COUNT as usize]>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let world_cache_light_data_new_lights = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_light_data_new_lights"),
            size: WORLD_CACHE_SIZE
                * size_of::<[u64; 1 + WORLD_CACHE_CELL_LIGHT_COUNT as usize]>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let world_cache_luminance_deltas = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_luminance_deltas"),
            size: WORLD_CACHE_SIZE * size_of::<f32>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let world_cache_active_cells_new_radiance =
            render_device.create_buffer(&BufferDescriptor {
                label: Some("solari_lighting_world_cache_active_cells_new_radiance"),
                size: WORLD_CACHE_SIZE * size_of::<[f32; 4]>() as u64,
                usage: BufferUsages::STORAGE,
                mapped_at_creation: false,
            });

        let world_cache_a = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_a"),
            size: WORLD_CACHE_SIZE * size_of::<u32>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });
        let world_cache_b = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_b"),
            size: 1024 * size_of::<u32>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let world_cache_active_cell_indices = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_active_cell_indices"),
            size: WORLD_CACHE_SIZE * size_of::<u32>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let world_cache_active_cells_count = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_active_cells_count"),
            size: size_of::<u32>() as u64,
            usage: BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        let world_cache_active_cells_dispatch = render_device.create_buffer(&BufferDescriptor {
            label: Some("solari_lighting_world_cache_active_cells_dispatch"),
            size: size_of::<[u32; 3]>() as u64,
            usage: BufferUsages::INDIRECT | BufferUsages::STORAGE,
            mapped_at_creation: false,
        });

        commands.entity(entity).insert(SolariLightingResources {
            light_tile_samples,
            gi_reservoirs_a,
            gi_reservoirs_b,
            world_cache_checksums,
            world_cache_life,
            world_cache_radiance,
            world_cache_geometry_data,
            world_cache_light_data,
            world_cache_light_data_new_lights,
            world_cache_luminance_deltas,
            world_cache_active_cells_new_radiance,
            world_cache_a,
            world_cache_b,
            world_cache_active_cell_indices,
            world_cache_active_cells_count,
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
