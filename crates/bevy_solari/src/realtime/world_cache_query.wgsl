#define_import_path bevy_solari::world_cache

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::{rand_f, rand_vec2f, rand_range_u}
#import bevy_render::maths::{PI, orthonormalize}
#import bevy_solari::brdf::evaluate_brdf
#import bevy_solari::realtime_bindings::{
    view_output, 
    light_tile_samples, 
    gi_reservoirs_a, 
    gbuffer, 
    depth_buffer,
    world_cache_checksums,
    world_cache_life, 
    world_cache_active_cells_count,
    world_cache_active_cell_indices,
    world_cache_geometry_data,
    world_cache_light_data,
    world_cache_light_data_new_lights,
    world_cache_radiance,
    world_cache_active_cells_new_radiance,
    view, 
    constants, 
    LightSamplePacked,
    WORLD_CACHE_CELL_LIGHT_COUNT,
    WorldCacheSingleLightData,
    WorldCacheLightDataRead,
}
#import bevy_solari::presample_light_tiles::unpack_light_sample
#import bevy_solari::sampling::{light_contribution_no_trace, select_random_light, select_random_light_inverse_pdf, trace_light_visibility, calculate_light_contribution}
#import bevy_solari::scene_bindings::ResolvedMaterial

/// How responsive the world cache is to changes in lighting (higher is less responsive, lower is more responsive)
const WORLD_CACHE_MAX_TEMPORAL_SAMPLES: f32 = 10.0;
/// Maximum amount of frames a cell can live for without being queried
const WORLD_CACHE_CELL_LIFETIME: u32 = 4u;
/// Maximum amount of attempts to find a cache entry after a hash collision
const WORLD_CACHE_MAX_SEARCH_STEPS: u32 = 3u;
/// Lights searched that aren't in the cell
const WORLD_CACHE_NEW_LIGHTS_SEARCH_COUNT_MIN: u32 = 4u;
const WORLD_CACHE_NEW_LIGHTS_SEARCH_COUNT_MAX: u32 = 8u;
const WORLD_CACHE_EXPLORATORY_SAMPLE_RATIO: f32 = 0.20;
const WORLD_CACHE_CELL_CONFIDENCE_LUM_MIN: f32 = 0.0001;
const WORLD_CACHE_CELL_CONFIDENCE_LUM_MAX: f32 = 0.1;

/// The size of a cache cell at the lowest LOD in meters
const WORLD_CACHE_POSITION_BASE_CELL_SIZE: f32 = 0.1;
/// How fast the world cache transitions between LODs as a function of distance to the camera
const WORLD_CACHE_POSITION_LOD_SCALE: f32 = 8.0;

/// Marker value for an empty cell
const WORLD_CACHE_EMPTY_CELL: u32 = 0u;

#ifndef WORLD_CACHE_NON_ATOMIC_LIFE_BUFFER
fn query_world_cache_radiance(rng: ptr<function, u32>, world_position: vec3<f32>, world_normal: vec3<f32>, view_position: vec3<f32>) -> vec3<f32> {
    let cell_size = get_cell_size(world_position, view_position);

    // https://tomclabault.github.io/blog/2025/regir, jitter_world_position_tangent_plane
    let TBN = orthonormalize(world_normal);
    let offset = (rand_vec2f(rng) * 2.0 - 1.0) * cell_size * 0.5;
    let jittered_position = world_position + offset.x * TBN[0] + offset.y * TBN[1];

    let world_position_quantized = quantize_position(jittered_position, cell_size);
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
            world_cache_geometry_data[key].world_position = jittered_position;
            world_cache_geometry_data[key].world_normal = world_normal;
            world_cache_light_data[key].visible_light_count = 0u;
            return vec3(0.0);
        } else {
            // Collision - linear probe to next entry
            key += 1u;
        }
    }

    return vec3(0.0);
}

fn query_world_cache_lights(rng: ptr<function, u32>, world_position: vec3<f32>, world_normal: vec3<f32>, view_position: vec3<f32>) -> WorldCacheLightDataRead {
    let cell_size = get_cell_size(world_position, view_position);

    // https://tomclabault.github.io/blog/2025/regir, jitter_world_position_tangent_plane
    let TBN = orthonormalize(world_normal);
    let offset = (rand_vec2f(rng) * 2.0 - 1.0) * cell_size * 0.5;
    let jittered_position = world_position + offset.x * TBN[0] + offset.y * TBN[1];

    var world_position_quantized = quantize_position(jittered_position, cell_size);
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
    let light_tile_start = subgroupBroadcastFirst(rand_range_u(128u, rng) * 1024u);
    var p = rand_f(rng);

    var selected = 0u;
    var selected_weight = 0.0;
    var weight_sum = 0.0;
    for (var i = 0u; i < samples; i++) {
        let tile_sample = light_tile_start + rand_range_u(1024u, rng);
        let sample = unpack_light_sample(light_tile_samples[tile_sample]);
        let direct_lighting = calculate_light_contribution(sample, world_position, world_normal);
        let brdf = evaluate_brdf(world_normal, wo, direct_lighting.wi, material);
        let radiance = direct_lighting.radiance * direct_lighting.inverse_pdf * brdf;

        let weight = log2(luminance(radiance) + 1.0);
        weight_sum += weight;

        let prob = weight / weight_sum;
        if p < prob {
            selected = sample.light_id;
            selected_weight = weight;
            p /= prob;
        } else {
            p = (p - prob) / (1.0 - prob);
        }
    }
    let base_pdf = f32(samples) / select_random_light_inverse_pdf(selected);
    return SelectedLight(selected, selected_weight, weight_sum, base_pdf);
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
