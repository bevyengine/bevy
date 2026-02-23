enable wgpu_ray_query;

#define_import_path bevy_solari::realtime_bindings

#import bevy_render::view::View
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_solari::sampling::LightSample

@group(1) @binding(0) var view_output: texture_storage_2d<rgba16float, read_write>;
@group(1) @binding(1) var<storage, read_write> light_tile_samples: array<LightSample>;
@group(1) @binding(2) var<storage, read_write> light_tile_resolved_samples: array<ResolvedLightSamplePacked>;
@group(1) @binding(3) var di_reservoirs_a: texture_storage_2d<rgba32uint, read_write>;
@group(1) @binding(4) var di_reservoirs_b: texture_storage_2d<rgba32uint, read_write>;
@group(1) @binding(5) var<storage, read_write> gi_reservoirs_a: array<Reservoir>;
@group(1) @binding(6) var<storage, read_write> gi_reservoirs_b: array<Reservoir>;
@group(1) @binding(7) var gbuffer: texture_2d<u32>;
@group(1) @binding(8) var depth_buffer: texture_depth_2d;
@group(1) @binding(9) var motion_vectors: texture_2d<f32>;
@group(1) @binding(10) var previous_gbuffer: texture_2d<u32>;
@group(1) @binding(11) var previous_depth_buffer: texture_depth_2d;
@group(1) @binding(12) var<uniform> view: View;
@group(1) @binding(13) var<uniform> previous_view: PreviousViewUniforms;

struct WorldCacheGeometryData {
    world_position: vec3<f32>,
    padding_a: u32,
    world_normal: vec3<f32>,
    padding_b: u32,
}
struct WorldCache {
    checksums: array<atomic<u32>, #{WORLD_CACHE_SIZE}>,
#ifdef WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER
    life: array<u32, #{WORLD_CACHE_SIZE}>,
#else
    life: array<atomic<u32>, #{WORLD_CACHE_SIZE}>,
#endif
    radiance: array<vec4<f32>, #{WORLD_CACHE_SIZE}>,
    geometry_data: array<WorldCacheGeometryData, #{WORLD_CACHE_SIZE}>,
    luminance_deltas: array<f32, #{WORLD_CACHE_SIZE}>,
    active_cells_new_radiance: array<vec3<f32>, #{WORLD_CACHE_SIZE}>,
    a: array<u32, #{WORLD_CACHE_SIZE}>,
    b: array<u32, 1024u>,
    active_cell_indices: array<u32, #{WORLD_CACHE_SIZE}>,
    active_cells_count: u32,
}
@group(1) @binding(14) var<storage, read_write> world_cache: WorldCache;

#ifdef DLSS_RR_GUIDE_BUFFERS
@group(2) @binding(0) var diffuse_albedo: texture_storage_2d<rgba8unorm, write>;
@group(2) @binding(1) var specular_albedo: texture_storage_2d<rgba8unorm, write>;
@group(2) @binding(2) var normal_roughness: texture_storage_2d<rgba16float, write>;
@group(2) @binding(3) var specular_motion_vectors: texture_storage_2d<rg16float, write>;
#endif

struct PushConstants { frame_index: u32, reset: u32 }
var<immediate> constants: PushConstants;

// Don't adjust the size of this struct without also adjusting `prepare::RESOLVED_LIGHT_SAMPLE_STRUCT_SIZE`.
struct ResolvedLightSamplePacked {
    world_position_x: f32,
    world_position_y: f32,
    world_position_z: f32,
    world_normal: u32,
    radiance: u32,
    inverse_pdf: f32,
}

// Don't adjust the size of this struct without also adjusting `prepare::GI_RESERVOIR_STRUCT_SIZE`.
struct Reservoir {
    sample_point_world_position: vec3<f32>,
    weight_sum: f32,
    radiance: vec3<f32>,
    confidence_weight: f32,
    sample_point_world_normal: vec3<f32>,
    unbiased_contribution_weight: f32,
}
