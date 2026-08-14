import bevy_render::view::View;
import bevy_pbr::prepass::bindings::PreviousViewUniforms;
import bevy_solari::scene::sampling::{LightSample, NULL_LIGHT_ID};

enable wgpu_ray_query;

@group(1) @binding(0) var view_output: texture_storage_2d<rgba16float, read_write>;
@group(1) @binding(1) var<storage, read_write> light_tile_samples: array<LightSample>;
@group(1) @binding(2) var<storage, read_write> light_tile_resolved_samples: array<ResolvedLightSamplePacked>;
@group(1) @binding(3) var<storage, read_write> reservoirs_a: array<Reservoir>;
@group(1) @binding(4) var<storage, read_write> reservoirs_b: array<Reservoir>;
@group(1) @binding(5) var gbuffer: texture_2d<u32>;
@group(1) @binding(6) var depth_buffer: texture_depth_2d;
@group(1) @binding(7) var motion_vectors: texture_2d<f32>;
@group(1) @binding(8) var previous_gbuffer: texture_2d<u32>;
@group(1) @binding(9) var previous_depth_buffer: texture_depth_2d;
@group(1) @binding(10) var<uniform> view: View;
@group(1) @binding(11) var<uniform> previous_view: PreviousViewUniforms;
@group(1) @binding(12) var<storage, read_write> world_cache: WorldCache;
@group(1) @binding(13) var<uniform> constants: SolariLightingSettings;

@if(DLSS_RR_GUIDE_BUFFERS) {
@group(2) @binding(0) var diffuse_albedo: texture_storage_2d<rgba8unorm, write>;
@group(2) @binding(1) var specular_albedo: texture_storage_2d<rgba8unorm, write>;
@group(2) @binding(2) var normal_roughness: texture_storage_2d<rgba16float, write>;
@group(2) @binding(3) var specular_motion_vectors: texture_storage_2d<rg16float, write>;
// Depth and motion vectors as seen through mirrors. These shadow the prepass pair for DLSS Ray
// Reconstruction only; the prepass originals stay authoritative for everything else, including
// ReSTIR's own temporal reprojection.
@group(2) @binding(4) var dlss_rr_depth: texture_storage_2d<r32float, write>;
@group(2) @binding(5) var dlss_rr_motion_vectors: texture_storage_2d<rg16float, write>;
}

// Roughness, as alpha, below which a surface gets a specular motion vector at all. About 0.25
// perceptual.
//
// Deliberately much looser than `MIRROR_ROUGHNESS_THRESHOLD`, and separate from it. Two different
// questions were previously answered with one number. Swapping a surface out demands its lobe really
// be delta, because a virtual position is meaningless otherwise. Describing how its reflection
// *moves* does not: it changes no light transport, and the reflection direction stands in for the
// lobe centre well past the delta range - NRD's dominant-direction fit is identically the mirror
// direction below 0.279 perceptual.
//
// Lives here rather than in `initial_path` because `resolve_dlss_rr_textures` needs the same number
// to decide which pixels' defaults it must leave alone, and importing back the other way would close
// a cycle.
const SPECULAR_GUIDE_ROUGHNESS_THRESHOLD = 0.0625;

// User-configurable settings from the `SolariLighting` component, plus per-frame
// state. Field order and types must match `SolariLightingUniforms` in `prepare.rs`.
struct SolariLightingSettings {
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

// Don't adjust the size of this struct without also adjusting `prepare::RESOLVED_LIGHT_SAMPLE_STRUCT_SIZE`.
struct ResolvedLightSamplePacked {
    world_position_x: f32,
    world_position_y: f32,
    world_position_z: f32,
    world_normal: u32,
    radiance: u32,
    inverse_pdf: f32,
}

// Don't adjust the size of this struct without also adjusting `prepare::RESERVOIR_STRUCT_SIZE`.
struct Reservoir {
    sample_point_world_position: vec3<f32>,
    unbiased_contribution_weight: f32,
    radiance: vec3<f32>,
    confidence_weight: f32,
    sample_point_world_normal: vec2<f32>,
    light_sample: LightSample,
}

fn empty_reservoir() -> Reservoir {
    return Reservoir(
        vec3(0.0),
        0.0,
        vec3(0.0),
        0.0,
        vec2(0.0),
        LightSample(NULL_LIGHT_ID, 0u),
    );
}

struct WorldCacheGeometryData {
    world_position: vec3<f32>,
    padding_a: u32,
    world_normal: vec3<f32>,
    padding_b: u32,
}

// Keep in sync with the packed world-cache layout constants in `prepare.rs`.
struct WorldCache {
    checksums: array<atomic<u32>, constants::WORLD_CACHE_SIZE>,
    @if(WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER)
    life: array<u32, constants::WORLD_CACHE_SIZE>,
    @if(!WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER)
    life: array<atomic<u32>, constants::WORLD_CACHE_SIZE>,
    radiance: array<vec4<f32>, constants::WORLD_CACHE_SIZE>,
    geometry_data: array<WorldCacheGeometryData, constants::WORLD_CACHE_SIZE>,
    luminance_deltas: array<f32, constants::WORLD_CACHE_SIZE>,
    active_cells_new_radiance: array<vec3<f32>, constants::WORLD_CACHE_SIZE>,
    a: array<u32, constants::WORLD_CACHE_SIZE>,
    b: array<u32, 1024u>,
    active_cell_indices: array<u32, constants::WORLD_CACHE_SIZE>,
    active_cells_count: u32,
}
