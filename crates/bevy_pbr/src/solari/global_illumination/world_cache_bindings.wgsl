#define_import_path bevy_solari::world_cache::bindings

/// Maximum amount of entries in the world cache (must be a power of 2)
const WORLD_CACHE_SIZE: u32 = 1048576u;
/// Maximum amount of frames a cell can live for without being queried
const WORLD_CACHE_CELL_LIFETIME: u32 = 30u;
/// Maximum amount of steps to linearly probe for on key collision before giving up
const WORLD_CACHE_MAX_SEARCH_STEPS: u32 = 10u;
/// Marker value for an empty cell
const WORLD_CACHE_EMPTY_CELL: u32 = 0u;

struct WorldCacheCellData {
    position: vec3<f32>,
    normal: vec3<f32>,
}

@group(#WORLD_CACHE_BIND_GROUP) @binding(0) var<storage, read_write> world_cache_checksums: array<atomic<u32>, WORLD_CACHE_SIZE>;

@group(#WORLD_CACHE_BIND_GROUP) @binding(1)
#ifdef WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER
var<storage, read_write> world_cache_life: array<u32, WORLD_CACHE_SIZE>;
#else
var<storage, read_write> world_cache_life: array<atomic<u32>, WORLD_CACHE_SIZE>;
#endif

@group(#WORLD_CACHE_BIND_GROUP) @binding(2) var<storage, read_write> world_cache_irradiance: array<vec4<f32>, WORLD_CACHE_SIZE>;
@group(#WORLD_CACHE_BIND_GROUP) @binding(3) var<storage, read_write> world_cache_cell_data: array<WorldCacheCellData, WORLD_CACHE_SIZE>;
@group(#WORLD_CACHE_BIND_GROUP) @binding(4) var<storage, read_write> world_cache_active_cells_new_irradiance: array<vec3<f32>, WORLD_CACHE_SIZE>;
@group(#WORLD_CACHE_BIND_GROUP) @binding(5) var<storage, read_write> world_cache_b1: array<u32, WORLD_CACHE_SIZE>;
@group(#WORLD_CACHE_BIND_GROUP) @binding(6) var<storage, read_write> world_cache_b2: array<u32, 1024u>;
@group(#WORLD_CACHE_BIND_GROUP) @binding(7) var<storage, read_write> world_cache_active_cell_indices: array<u32, WORLD_CACHE_SIZE>;
@group(#WORLD_CACHE_BIND_GROUP) @binding(8) var<storage, read_write> world_cache_active_cells_count: u32;

#ifndef EXCLUDE_WORLD_CACHE_ACTIVE_CELLS_DISPATCH
@group(#WORLD_CACHE_BIND_GROUP) @binding(9) var<storage, read_write> world_cache_active_cells_dispatch: vec3<u32>;
#endif
