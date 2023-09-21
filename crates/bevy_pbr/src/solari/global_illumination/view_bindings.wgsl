#define_import_path bevy_solari::global_illumination::view_bindings

#import bevy_render::view View

struct SphericalHarmonicsPacked {
    a: vec4<f32>,
    b: vec4<f32>,
    c: vec4<f32>,
    d: vec4<f32>,
    e: vec4<f32>,
    f: vec4<f32>,
    g: vec4<f32>,
}

struct WorldCacheCellData {
    position: vec3<f32>,
    normal: vec3<f32>,
}

/// Maximum amount of entries in the world cache (must be a power of 2)
const WORLD_CACHE_SIZE: u32 = 1048576u;
/// Marker value for an empty cell
const WORLD_CACHE_EMPTY_CELL: u32 = 0u;

/// Maximum amount of frames a cell can live for without being queried
const WORLD_CACHE_CELL_LIFETIME: u32 = 30u;
/// Maximum amount of steps to linearly probe for on key collision before giving up
const WORLD_CACHE_MAX_SEARCH_STEPS: u32 = 10u;
/// TODO: Docs
const WORLD_CACHE_DISCRETIZATION_FACTOR: u32 = 2u;
/// TODO: Docs
const FIRST_RADIANCE_CASCADE_INTERVAL: f32 = 0.5;

@group(1) @binding(0) var<uniform> view: View;
@group(1) @binding(1) var previous_depth_buffer: texture_depth_2d;
@group(1) @binding(2) var depth_buffer: texture_depth_2d;
@group(1) @binding(3) var normals_buffer: texture_2d<f32>;
@group(1) @binding(4) var motion_vectors: texture_2d<f32>;
@group(1) @binding(5) var screen_probes_history: texture_2d_array<f32>;
@group(1) @binding(6) var screen_probes: texture_storage_2d_array<rgba16float, read_write>;
@group(1) @binding(7) var screen_probes_confidence_history: texture_2d_array<u32>;
@group(1) @binding(8) var screen_probes_confidence: texture_storage_2d_array<r8uint, write>;
@group(1) @binding(9) var screen_probes_merge_buffer: texture_storage_2d_array<rgba16float, read_write>;
@group(1) @binding(10) var<storage, read_write> screen_probes_spherical_harmonics: array<SphericalHarmonicsPacked>;
@group(1) @binding(11) var diffuse_irradiance_output: texture_storage_2d<rgba16float, write>;
@group(1) @binding(12) var<storage, read_write> world_cache_checksums: array<atomic<u32>, WORLD_CACHE_SIZE>;
@group(1) @binding(13)
#ifdef WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER
var<storage, read_write> world_cache_life: array<u32, WORLD_CACHE_SIZE>;
#else
var<storage, read_write> world_cache_life: array<atomic<u32>, WORLD_CACHE_SIZE>;
#endif
@group(1) @binding(14) var<storage, read_write> world_cache_irradiance: array<vec4<f32>, WORLD_CACHE_SIZE>;
@group(1) @binding(15) var<storage, read_write> world_cache_cell_data: array<WorldCacheCellData, WORLD_CACHE_SIZE>;
@group(1) @binding(16) var<storage, read_write> world_cache_active_cells_new_irradiance: array<vec3<f32>, WORLD_CACHE_SIZE>;
@group(1) @binding(17) var<storage, read_write> world_cache_a: array<u32, WORLD_CACHE_SIZE>;
@group(1) @binding(18) var<storage, read_write> world_cache_b: array<u32, 1024u>;
@group(1) @binding(19) var<storage, read_write> world_cache_active_cell_indices: array<u32, WORLD_CACHE_SIZE>;
@group(1) @binding(20) var<storage, read_write> world_cache_active_cells_count: u32;
#ifdef INCLUDE_WORLD_CACHE_ACTIVE_CELLS_DISPATCH
@group(1) @binding(21) var<storage, read_write> world_cache_active_cells_dispatch: vec3<u32>;
#endif
