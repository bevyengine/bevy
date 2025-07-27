#define_import_path bevy_solari::world_cache

/// Maximum amount of frames a cell can live for without being queried
const WORLD_CACHE_CELL_LIFETIME: u32 = 30u;
/// Maximum amount of attempts to find a cache entry after a hash collision
const WORLD_CACHE_MAX_SEARCH_STEPS: u32 = 10u;

/// Controls the base size of each cache cell
const WORLD_CACHE_POSITION_DISCRETIZATION_FACTOR: f32 = 2.0;
/// Controls the normal quantization of each cell
const WORLD_CACHE_NORMAL_DISCRETIZATION_FACTOR: f32 = 2.0;

/// Marker value for an empty cell
const WORLD_CACHE_EMPTY_CELL: u32 = 0u;

#ifdef WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER
alias world_cache_life_type = u32;
#else
alias world_cache_life_type = atomic<u32>;
#endif

struct WorldCacheGeometryData {
    position: vec3<f32>,
    padding1: u32,
    normal: vec3<f32>,
    padding2: u32
}

@group(1) @binding(14) var<storage, read_write> world_cache_checksums: array<atomic<u32>, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(15) var<storage, read_write> world_cache_life: array<world_cache_life_type, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(16) var<storage, read_write> world_cache_radiance: array<vec4<f32>, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(17) var<storage, read_write> world_cache_geometry_data: array<WorldCacheGeometryData, #{WORLD_CACHE_SIZE}>;

fn query_world_cache(world_position: vec3<f32>, world_normal: vec3<f32>) -> vec3<f32> {
    var key = compute_key(world_position, world_normal);
    let checksum = compute_checksum(world_position, world_normal);

    for (var i = 0u; i < WORLD_CACHE_MAX_SEARCH_STEPS; i++) {
        let existing_checksum = atomicCompareExchangeWeak(&world_cache_checksums(key), WORLD_CACHE_EMPTY_CELL, checksum).old_value;.
        if existing_checksum == checksum {
            // Cache entry already exists - get radiance and reset cell lifetime
            atomicStore(&world_cache_life[key], WORLD_CACHE_CELL_LIFETIME);
            return world_cache_radiance[key].rgb;
        } else if existing_checksum == WORLD_CACHE_EMPTY_CELL {
            // Cell is empty - reset cell lifetime so that it starts getting updated next frame
            atomicStore(&world_cache_life[key], WORLD_CACHE_CELL_LIFETIME);
            world_cache_geometry_data[key].position = world_position;
            world_cache_geometry_data[key].normal = world_normal;
            return vec3(0.0);
        } else {
            // Collision - jump to another entry
            key = wrap_key(pcg_hash(key));
        }
    }

    return vec3(0.0);
}

fn compute_key(world_position: vec3<f32>, world_normal: vec3<f32>) -> u32 {
    let world_position_quantized = bitcast<vec3<u32>>(quantize_position(world_position));
    let world_normal_quantized = bitcast<vec3<u32>>(quantize_normal(world_normal));
    var key = pcg_hash(world_position_quantized.x);
    key = pcg_hash(key + world_position_quantized.y);
    key = pcg_hash(key + world_position_quantized.z);
    key = pcg_hash(key + world_normal_quantized.x);
    key = pcg_hash(key + world_normal_quantized.y);
    key = pcg_hash(key + world_normal_quantized.z);
    return wrap_key(key);
}

fn compute_checksum(world_position: vec3<f32>, world_normal: vec3<f32>) -> u32 {
    let world_position_quantized = bitcast<vec3<u32>>(quantize_position(world_position));
    let world_normal_quantized = bitcast<vec3<u32>>(quantize_normal(world_normal));
    var key = iqint_hash(world_position_quantized.x);
    key = iqint_hash(key + world_position_quantized.y);
    key = iqint_hash(key + world_position_quantized.z);
    key = iqint_hash(key + world_normal_quantized.x);
    key = iqint_hash(key + world_normal_quantized.y);
    key = iqint_hash(key + world_normal_quantized.z);
    return key;
}

fn quantize_position(world_position: vec3<f32>) -> vec3<f32> {
    return floor(world_position / WORLD_CACHE_POSITION_DISCRETIZATION_FACTOR);
}

fn quantize_normal(world_normal: vec3<f32>) -> vec3<f32> {
    return floor(world_normal / WORLD_CACHE_NORMAL_DISCRETIZATION_FACTOR);
}

fn pcg_hash(input: u32) -> u32 {
    let state = input * 747796405u + 2891336453u;
    let word = ((state >> ((state >> 28u) + 4u)) ^ state) * 277803737u;
    return (word >> 22u) ^ word;
}

fn iqint_hash(input: u32) -> u32 {
    let n = (input << 13u) ^ input;
    return n * (n * n * 15731u + 789221u) + 1376312589u;
}

fn wrap_key(key: u32) -> u32 {
    return key & (WORLD_CACHE_SIZE - 1u);
}
