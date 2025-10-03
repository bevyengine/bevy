#define_import_path bevy_solari::world_cache

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::rand_f
#import bevy_render::maths::PI
// #import bevy_solari::brdf::evaluate_brdf
#import bevy_solari::sampling::{light_contribution_no_trace, select_random_light, select_random_light_inverse_pdf, trace_light_visibility}
#import bevy_solari::scene_bindings::ResolvedMaterial

/// How responsive the world cache is to changes in lighting (higher is less responsive, lower is more responsive)
const WORLD_CACHE_MAX_TEMPORAL_SAMPLES: f32 = 20.0;
/// Maximum amount of frames a cell can live for without being queried
const WORLD_CACHE_CELL_LIFETIME: u32 = 30u;
/// Maximum amount of attempts to find a cache entry after a hash collision
const WORLD_CACHE_MAX_SEARCH_STEPS: u32 = 3u;
/// Maximum lights stored in each cache cell
/// This should match `WORLD_CACHE_CELL_LIGHT_COUNT` in `realtime/prepare.rs`!
const WORLD_CACHE_CELL_LIGHT_COUNT: u32 = 8u;
/// Lights searched that aren't in the cell
const WORLD_CACHE_NEW_LIGHTS_SEARCH_COUNT_MIN: u32 = 4u;
const WORLD_CACHE_NEW_LIGHTS_SEARCH_COUNT_MAX: u32 = 10u;
const WORLD_CACHE_EXPLORATORY_SAMPLE_RATIO: f32 = 0.20;
const WORLD_CACHE_CELL_CONFIDENCE_LUM_MIN: f32 = 0.0001;
const WORLD_CACHE_CELL_CONFIDENCE_LUM_MAX: f32 = 0.1;

/// The size of a cache cell at the lowest LOD in meters
const WORLD_CACHE_POSITION_BASE_CELL_SIZE: f32 = 0.1;
/// How fast the world cache transitions between LODs as a function of distance to the camera
const WORLD_CACHE_POSITION_LOD_SCALE: f32 = 30.0;

/// Marker value for an empty cell
const WORLD_CACHE_EMPTY_CELL: u32 = 0u;

struct WorldCacheGeometryData {
    world_position: vec3<f32>,
    padding_a: u32,
    world_normal: vec3<f32>,
    padding_b: u32
}

struct WorldCacheSingleLightData {
    light: u32,
    weight: f32,
}

struct WorldCacheLightDataRead {
    visible_light_count: u32,
    padding: u32,
    visible_lights: array<WorldCacheSingleLightData, WORLD_CACHE_CELL_LIGHT_COUNT>,
}

struct WorldCacheLightDataWrite {
    visible_light_count: atomic<u32>,
    padding: u32,
    visible_lights: array<atomic<u64>, WORLD_CACHE_CELL_LIGHT_COUNT>,
}

@group(1) @binding(10) var<storage, read_write> world_cache_checksums: array<atomic<u32>, #{WORLD_CACHE_SIZE}>;
#ifdef WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER
@group(1) @binding(11) var<storage, read_write> world_cache_life: array<u32, #{WORLD_CACHE_SIZE}>;
#else
@group(1) @binding(11) var<storage, read_write> world_cache_life: array<atomic<u32>, #{WORLD_CACHE_SIZE}>;
#endif
@group(1) @binding(12) var<storage, read_write> world_cache_radiance: array<vec4<f32>, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(13) var<storage, read_write> world_cache_geometry_data: array<WorldCacheGeometryData, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(14) var<storage, read_write> world_cache_light_data: array<WorldCacheLightDataRead, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(15) var<storage, read_write> world_cache_light_data_new_lights: array<WorldCacheLightDataWrite, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(16) var<storage, read_write> world_cache_active_cells_new_radiance: array<vec3<f32>, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(17) var<storage, read_write> world_cache_a: array<u32, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(18) var<storage, read_write> world_cache_b: array<u32, 1024u>;
@group(1) @binding(19) var<storage, read_write> world_cache_active_cell_indices: array<u32, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(20) var<storage, read_write> world_cache_active_cells_count: u32;

#ifndef WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER
fn query_world_cache_radiance(world_position: vec3<f32>, world_normal: vec3<f32>, view_position: vec3<f32>) -> vec3<f32> {
    let cell_size = get_cell_size(world_position, view_position);
    let world_position_quantized = quantize_position(world_position, cell_size);
    let world_normal_quantized = quantize_normal(world_normal);
    var key = compute_key(world_position_quantized, world_normal_quantized);
    let checksum = compute_checksum(world_position_quantized, world_normal_quantized);

    for (var i = 0u; i < WORLD_CACHE_MAX_SEARCH_STEPS; i++) {
        let existing_checksum = atomicCompareExchangeWeak(&world_cache_checksums[key], WORLD_CACHE_EMPTY_CELL, checksum).old_value;
        if existing_checksum == checksum {
            // Cache entry already exists - get radiance and reset cell lifetime
            atomicStore(&world_cache_life[key], WORLD_CACHE_CELL_LIFETIME);
            return world_cache_radiance[key].rgb;
        } else if existing_checksum == WORLD_CACHE_EMPTY_CELL {
            // Cell is empty - reset cell lifetime so that it starts getting updated next frame
            atomicStore(&world_cache_life[key], WORLD_CACHE_CELL_LIFETIME);
            world_cache_geometry_data[key].world_position = world_position;
            world_cache_geometry_data[key].world_normal = world_normal;
            world_cache_light_data[key].visible_light_count = 0u;
            return vec3(0.0);
        } else {
            // Collision - jump to another entry
            key = wrap_key(pcg_hash(key));
        }
    }

    return vec3(0.0);
}

fn query_world_cache_lights(rng: ptr<function, u32>, world_position: vec3<f32>, world_normal: vec3<f32>, view_position: vec3<f32>) -> WorldCacheLightDataRead {
    let cell_size = get_cell_size(world_position, view_position);
    var world_position_quantized = quantize_position(world_position, cell_size);
    let center_offset = quantize_position_fract(world_position, cell_size) - vec3(0.5);
    let direction = vec3<i32>(sign(center_offset));
    let lerp = vec3(rand_f(rng), rand_f(rng), rand_f(rng));
    let p_lerp_away = abs(center_offset);
    world_position_quantized += select(vec3(0), direction, lerp > p_lerp_away);

    let world_normal_quantized = quantize_normal(world_normal);
    var key = compute_key(world_position_quantized, world_normal_quantized);
    let checksum = compute_checksum(world_position_quantized, world_normal_quantized);

    for (var i = 0u; i < WORLD_CACHE_MAX_SEARCH_STEPS; i++) {
        let existing_checksum = atomicCompareExchangeWeak(&world_cache_checksums[key], WORLD_CACHE_EMPTY_CELL, checksum).old_value;
        if existing_checksum == checksum {
            // Cache entry already exists - get radiance and reset cell lifetime
            atomicStore(&world_cache_life[key], WORLD_CACHE_CELL_LIFETIME);
            return world_cache_light_data[key];
        } else if existing_checksum == WORLD_CACHE_EMPTY_CELL {
            // Cell is empty - reset cell lifetime so that it starts getting updated next frame
            atomicStore(&world_cache_life[key], WORLD_CACHE_CELL_LIFETIME);
            world_cache_geometry_data[key].world_position = world_position;
            world_cache_geometry_data[key].world_normal = world_normal;
            world_cache_light_data[key].visible_light_count = 0u;
            return WorldCacheLightDataRead(0u, 0u, array<WorldCacheSingleLightData, WORLD_CACHE_CELL_LIGHT_COUNT>());
        } else {
            // Collision - jump to another entry
            key = wrap_key(pcg_hash(key));
        }
    }

    return WorldCacheLightDataRead(0u, 0u, array<WorldCacheSingleLightData, WORLD_CACHE_CELL_LIGHT_COUNT>());
}

fn write_world_cache_light(cell: EvaluatedLighting, world_position: vec3<f32>, world_normal: vec3<f32>, view_position: vec3<f32>, exposure: f32) {
    let cell_selected_weight = cell.data.weight + log2(exposure);
    if cell_selected_weight < WORLD_CACHE_CELL_CONFIDENCE_LUM_MIN { return; }    

    let cell_size = get_cell_size(world_position, view_position);
    let world_position_quantized = quantize_position(world_position, cell_size);
    let world_normal_quantized = quantize_normal(world_normal);
    var key = compute_key(world_position_quantized, world_normal_quantized);
    let checksum = compute_checksum(world_position_quantized, world_normal_quantized);

    for (var i = 0u; i < WORLD_CACHE_MAX_SEARCH_STEPS; i++) {
        let existing_checksum = atomicCompareExchangeWeak(&world_cache_checksums[key], WORLD_CACHE_EMPTY_CELL, checksum).old_value;
        if existing_checksum == checksum || existing_checksum == WORLD_CACHE_EMPTY_CELL {
            let index = atomicAdd(&world_cache_light_data_new_lights[key].visible_light_count, 1u) & (WORLD_CACHE_CELL_LIGHT_COUNT - 1u);
            let packed = (u64(bitcast<u32>(cell.data.weight)) << 32u) | u64(cell.data.light);
            atomicStore(&world_cache_light_data_new_lights[key].visible_lights[index], packed);

            if existing_checksum == WORLD_CACHE_EMPTY_CELL {
                // Cell is empty - reset cell lifetime so that it starts getting updated next frame
                atomicStore(&world_cache_life[key], WORLD_CACHE_CELL_LIFETIME);
                world_cache_geometry_data[key].world_position = world_position;
                world_cache_geometry_data[key].world_normal = world_normal;
                world_cache_light_data[key].visible_light_count = 0u;
            }
        } else  {
            // Collision - jump to another entry
            key = wrap_key(pcg_hash(key));
        }
    }
}
#endif

struct EvaluatedLighting {
    radiance: vec3<f32>,
    inverse_pdf: f32,
    data: WorldCacheSingleLightData,
}

fn evaluate_lighting_from_cache(
    rng: ptr<function, u32>, 
    cell: WorldCacheLightDataRead,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    wo: vec3<f32>,
    material: ResolvedMaterial,
    exposure: f32,
) -> EvaluatedLighting {
    let cell_selected_light = select_light_from_cache_cell(rng, cell, world_position, world_normal, wo, material);
    let cell_selected_weight = cell_selected_light.weight + log2(exposure);
    let cell_confidence = smoothstep(WORLD_CACHE_CELL_CONFIDENCE_LUM_MIN, WORLD_CACHE_CELL_CONFIDENCE_LUM_MAX, cell_selected_weight);

    // Sample more random lights if our cell has bad lights
    let random_sample_count = u32(round(mix(f32(WORLD_CACHE_NEW_LIGHTS_SEARCH_COUNT_MAX), f32(WORLD_CACHE_NEW_LIGHTS_SEARCH_COUNT_MIN), cell_confidence)));
    let random_selected_light = select_light_randomly(rng, world_position, world_normal, wo, material, random_sample_count);

    let p_cell_selection = select(p_wrs(cell_selected_light), 0.0, cell_selected_light.weight_sum < 0.0001);
    let p_random_selection = select(p_wrs(random_selected_light), 0.0, random_selected_light.weight_sum < 0.0001);
    let p_random_selection_clamped = min(mix(1.0, WORLD_CACHE_EXPLORATORY_SAMPLE_RATIO * p_cell_selection, cell_confidence), p_random_selection);

    let weight_sum = p_cell_selection + p_random_selection_clamped;
    if weight_sum < 0.0001 {
        return EvaluatedLighting(vec3(0.0), 0.0, WorldCacheSingleLightData(0, 0.0));
    }

    let p_should_choose_cell = p_cell_selection / weight_sum;
    let p_should_choose_random = 1.0 - p_should_choose_cell;
    var sel: u32;
    var sel_weight: f32;
    var pdf: f32;
    if rand_f(rng) < p_should_choose_cell {
        sel = cell_selected_light.light;
        sel_weight = cell_selected_light.weight;
        pdf = p_should_choose_cell * p_cell_selection;
    } else {
        sel = random_selected_light.light;
        sel_weight = random_selected_light.weight;
        pdf = p_should_choose_random * p_random_selection;
    }
    
    // TODO: reuse the eval that we did for light selection somehow
    let direct_lighting = light_contribution_no_trace(rng, sel, world_position, world_normal);
    let brdf = evaluate_brdf(world_normal, wo, direct_lighting.wi, material);
    let visibility = trace_light_visibility(world_position, direct_lighting.world_position);
    let radiance = direct_lighting.radiance * brdf * visibility;
    let inverse_pdf = direct_lighting.inverse_pdf / pdf;
    return EvaluatedLighting(radiance, inverse_pdf, WorldCacheSingleLightData(sel, sel_weight * visibility));
}

struct SelectedLight {
    light: u32,
    weight: f32,
    weight_sum: f32,
    base_pdf: f32,
}

fn p_wrs(selection: SelectedLight) -> f32 {
    return (selection.weight / selection.weight_sum) * selection.base_pdf;
}

fn select_light_from_cache_cell(
    rng: ptr<function, u32>, 
    cell: WorldCacheLightDataRead,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    wo: vec3<f32>,
    material: ResolvedMaterial
) -> SelectedLight {
    var p = rand_f(rng);
    
    var selected = 0u;
    var selected_weight = 0.0;
    var weight_sum = 0.0;
    // WRS to select the light based on unshadowed contribution
    for (var i = 0u; i < cell.visible_light_count; i++) {
        let light_id = cell.visible_lights[i].light;
        let direct_lighting = light_contribution_no_trace(rng, light_id, world_position, world_normal);
        let brdf = evaluate_brdf(world_normal, wo, direct_lighting.wi, material);
        // Weight by inverse_pdf to bias towards larger triangles
        let radiance = direct_lighting.radiance * direct_lighting.inverse_pdf * brdf;

        let weight = log2(luminance(radiance) + 1.0);
        weight_sum += weight;

        let prob = weight / weight_sum;
        if p < prob {
            selected = light_id;
            selected_weight = weight;
            p /= prob;
        } else {
            p = (p - prob) / (1.0 - prob);
        }
    }
    return SelectedLight(selected, selected_weight, weight_sum, 1.0);
}

fn select_light_randomly(
    rng: ptr<function, u32>, 
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    wo: vec3<f32>,
    material: ResolvedMaterial,
    samples: u32,
) -> SelectedLight { 
    var p = rand_f(rng);

    var selected = 0u;
    var selected_weight = 0.0;
    var weight_sum = 0.0;
    for (var i = 0u; i < samples; i++) {
        let light_id = select_random_light(rng);
        let direct_lighting = light_contribution_no_trace(rng, light_id, world_position, world_normal);
        let brdf = evaluate_brdf(world_normal, wo, direct_lighting.wi, material);
        let radiance = direct_lighting.radiance * direct_lighting.inverse_pdf * brdf;

        let weight = log2(luminance(radiance) + 1.0);
        weight_sum += weight;

        let prob = weight / weight_sum;
        if p < prob {
            selected = light_id;
            selected_weight = weight;
            p /= prob;
        } else {
            p = (p - prob) / (1.0 - prob);
        }
    }
    let base_pdf = f32(samples) / select_random_light_inverse_pdf(selected);
    return SelectedLight(selected, selected_weight, weight_sum, base_pdf);
}

fn evaluate_brdf(normal: vec3<f32>, wo: vec3<f32>, wi: vec3<f32>, material: ResolvedMaterial) -> vec3<f32> {
    return material.base_color / PI;
}

fn get_cell_size(world_position: vec3<f32>, view_position: vec3<f32>) -> f32 {
    let camera_distance = distance(view_position, world_position) / WORLD_CACHE_POSITION_LOD_SCALE;
    let lod = exp2(floor(log2(1.0 + camera_distance)));
    return WORLD_CACHE_POSITION_BASE_CELL_SIZE * lod;
}

fn quantize_position(world_position: vec3<f32>, quantization_factor: f32) -> vec3<i32> {
    return vec3<i32>(floor(world_position / quantization_factor));
}

fn quantize_position_fract(world_position: vec3<f32>, quantization_factor: f32) -> vec3<f32> {
    return fract(world_position / quantization_factor);
}

fn quantize_normal(world_normal: vec3<f32>) -> vec3<i32> {
    let x = vec3(1.0, 0.0, 0.0);
    let y = vec3(0.0, 1.0, 0.0);
    let z = vec3(0.0, 0.0, 1.0);
    let dot_x = dot(world_normal, x);
    let dot_y = dot(world_normal, y);
    let dot_z = dot(world_normal, z);
    let max_dot = max(max(abs(dot_x), abs(dot_y)), abs(dot_z));

    var sel = select(vec3(0.0), x, max_dot == dot_x);
    sel = select(sel, y, max_dot == dot_y);
    sel = select(sel, z, max_dot == dot_z);

    return vec3<i32>(sel * sign(max_dot));
}

// TODO: Clustering
fn compute_key(world_position: vec3<i32>, world_normal: vec3<i32>) -> u32 {
    let pos = vec3<u32>(world_position);
    let norm = vec3<u32>(world_normal);
    var key = pcg_hash(pos.x);
    key = pcg_hash(key + pos.y);
    key = pcg_hash(key + pos.z);
    key = pcg_hash(key + norm.x);
    key = pcg_hash(key + norm.y);
    key = pcg_hash(key + norm.z);
    return wrap_key(key);
}

fn compute_checksum(world_position: vec3<i32>, world_normal: vec3<i32>) -> u32 {
    let pos = vec3<u32>(world_position);
    let norm = vec3<u32>(world_normal);
    var key = iqint_hash(pos.x);
    key = iqint_hash(key + pos.y);
    key = iqint_hash(key + pos.z);
    key = iqint_hash(key + norm.x);
    key = iqint_hash(key + norm.y);
    key = iqint_hash(key + norm.z);
    return u32(key);
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
    return key & (#{WORLD_CACHE_SIZE} - 1u);
}
