enable wgpu_ray_query;

#define_import_path bevy_solari::light_cache_query

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::{rand_f, rand_vec2f, rand_range_u, rand_u}
#import bevy_solari::brdf::evaluate_brdf
#import bevy_solari::gbuffer_utils::{gpixel_resolve, pixel_dissimilar}
#import bevy_solari::scene_bindings::{light_sources, LIGHT_SOURCE_KIND_DIRECTIONAL, ResolvedMaterial}
#import bevy_solari::sampling::{calculate_resolved_light_contribution, resolve_and_calculate_light_contribution, resolve_light_sample, trace_light_visibility, LightSample}
#import bevy_solari::presample_light_tiles::unpack_resolved_light_sample
#import bevy_solari::realtime_bindings::{
    light_tile_samples, 
    light_tile_resolved_samples,
    gbuffer,
    depth_buffer,
    motion_vectors, 
    previous_gbuffer, 
    previous_depth_buffer, 
    light_cache, 
    view,
    previous_view,
    constants, 
    WeightedLight, 
    LightCacheCell
}

/// Lights searched that aren't in the cell
const LIGHT_CACHE_NEW_LIGHTS_SEARCH_COUNT_MIN: u32 = 2u;
const LIGHT_CACHE_NEW_LIGHTS_SEARCH_COUNT_MAX: u32 = 8u;
const LIGHT_CACHE_EXPLORATORY_SAMPLE_RATIO_MIN: f32 = 0.25;
const LIGHT_CACHE_EXPLORATORY_SAMPLE_RATIO_MAX: f32 = 1.0;
const LIGHT_CACHE_CELL_CONFIDENCE_LUM_MIN: f32 = 0.1;
const LIGHT_CACHE_CELL_CONFIDENCE_LUM_MAX: f32 = 0.3;

struct LoadedCell {
    cell: LightCacheCell,
    exploratory_sample_ratio: f32,
}

fn empty_cell() -> LightCacheCell {
    return LightCacheCell(0u, array<WeightedLight, #{LIGHT_CACHE_LIGHTS_PER_CELL}>());
}

fn load_light_cache_cell(rng: ptr<function, u32>, pixel_id: vec2<u32>) -> LoadedCell {
    // If reset, clear all history
    if bool(constants.reset) {
        return LoadedCell(empty_cell(), LIGHT_CACHE_EXPLORATORY_SAMPLE_RATIO_MAX);
    }

    let depth = textureLoad(depth_buffer, pixel_id, 0);
    let surface = gpixel_resolve(textureLoad(gbuffer, pixel_id, 0), depth, pixel_id, view.main_pass_viewport.zw, view.world_from_clip);
    let pixel_id_float = vec2<f32>(pixel_id);
    let motion_vector = textureLoad(motion_vectors, pixel_id, 0).xy;
    var sel_pixel_id_float = round(pixel_id_float - motion_vector * view.main_pass_viewport.zw);
    var exploratory_sample_ratio = LIGHT_CACHE_EXPLORATORY_SAMPLE_RATIO_MIN;

    // If off-screen or pixel is drastically different, fall back to current pixel
    if any(sel_pixel_id_float < vec2(0.0)) || any(sel_pixel_id_float >= view.main_pass_viewport.zw) {
        sel_pixel_id_float = pixel_id_float;
        exploratory_sample_ratio = LIGHT_CACHE_EXPLORATORY_SAMPLE_RATIO_MAX;
    } else {
        let pixel_id = vec2<u32>(sel_pixel_id_float);
        let temporal_depth = textureLoad(previous_depth_buffer, pixel_id, 0);
        let temporal_surface = gpixel_resolve(textureLoad(previous_gbuffer, pixel_id, 0), temporal_depth, pixel_id, view.main_pass_viewport.zw, previous_view.world_from_clip);
        if pixel_dissimilar(depth, surface.world_position, temporal_surface.world_position, surface.world_normal, temporal_surface.world_normal, view) {
            sel_pixel_id_float = pixel_id_float;
            exploratory_sample_ratio = LIGHT_CACHE_EXPLORATORY_SAMPLE_RATIO_MAX;
        }
    }

    let cell_id_float = sel_pixel_id_float / 8.0;
    let bilinear_weights = fract(cell_id_float) - 0.5;
    let p_lerp = abs(bilinear_weights);
    let lerp = rand_vec2f(rng);
    let direction = sign(bilinear_weights);
    let lerp_offset = select(vec2(0.0), direction, lerp > p_lerp);

    let base_cell_id = floor(cell_id_float);
    let cell_id  = vec2<u32>(base_cell_id + lerp_offset);
    let max_cell = vec2<u32>(view.main_pass_viewport.zw) >> vec2(3u);
    let clamped_cell_id = clamp(cell_id, vec2(0u), max_cell);
    let cell_index = clamped_cell_id.x + clamped_cell_id.y * (max_cell.x + 1u);
    let cell = light_cache[cell_index];

    // If the lerped cell is empty, try falling back to the base cell
    if cell.visible_light_count == 0u && any(lerp_offset != vec2(0.0)) {
        let base_cell_index = vec2<u32>(base_cell_id);
        let base_cell_index_flat = base_cell_index.x + base_cell_index.y * (max_cell.x + 1u);
        return LoadedCell(light_cache[base_cell_index_flat], LIGHT_CACHE_EXPLORATORY_SAMPLE_RATIO_MAX);
    }
    return LoadedCell(cell, exploratory_sample_ratio);
}

var<workgroup> sort_temp: array<u64, 64>;

fn compare_and_swap(local_index: u32, idx: vec2<u32>) {
    if sort_temp[idx.x] < sort_temp[idx.y] && local_index < 32u {
        let temp = sort_temp[idx.x];
        sort_temp[idx.x] = sort_temp[idx.y];
        sort_temp[idx.y] = temp;
    }
}

fn flip(t: u32, h: u32) {
    let q = ((t << 1u) >> h) << h;
    let m = (1u << h) - 1u ;
    let b = t & m;
    compare_and_swap(t, q + vec2<u32>(b, (1u << h) - b));
}

fn disperse(t: u32, h: u32) {
    let q = ((t << 1u) >> h) << h;
    let m = (1u << h) - 1u;
    let b = t & m;
    compare_and_swap(t, q + vec2<u32>(b, b + (1u << (h - 1u))));
}

fn sort(local_index: u32, data: u64) {
    sort_temp[local_index] = data;
    workgroupBarrier();

    for (var h = 1u; h <= 6u; h += 1u) {
        flip(local_index, h);
        workgroupBarrier();
        for (var hh = h - 1u; hh > 0u; hh -= 1u) {
            disperse(local_index, hh);
            workgroupBarrier();
        }
    }
}

fn write_light_cache_cell(pixel_id: vec2<u32>, local_index: u32, light: WeightedLight) {
    let data = u64(bitcast<u32>(max(light.weight, 0.0))) << 32u | u64(light.light);
    sort(local_index, data);

    if local_index != 0u {
        return;
    }

    var light_count = 0u;
    var lights: array<WeightedLight, #{LIGHT_CACHE_LIGHTS_PER_CELL}>;
    for (var i = 0u; i < 64; i++) {
        let data = sort_temp[i];
        let light = u32(data);
        let weight = bitcast<f32>(u32(data >> 32u));
        var already_exists = false;
        for (var j = 0u; j < light_count; j++) {
            if light == lights[j].light {
                already_exists = true;
                break;
            }
        }
        if already_exists {
            continue;
        }

        if weight > LIGHT_CACHE_CELL_CONFIDENCE_LUM_MIN && light_count < #{LIGHT_CACHE_LIGHTS_PER_CELL} {
            lights[light_count] = WeightedLight(light, weight);
            light_count += 1u;
        } else {
            break;
        }
    }

    let cell_id = pixel_id >> vec2(3u);
    let max_cell = vec2<u32>(view.main_pass_viewport.zw) >> vec2(3u);
    let cell_index = cell_id.x + cell_id.y * (max_cell.x + 1u);
    light_cache[cell_index] = LightCacheCell(light_count, lights);
}

struct EvaluatedLighting {
    light_sample: LightSample,
    radiance: vec3<f32>,
    inverse_pdf: f32,
    wi: vec3<f32>,
    brdf_rays_can_hit: bool,
}

fn evaluate_lighting(
    rng: ptr<function, u32>,
    pixel_id: vec2<u32>,
    local_index: u32,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    wo: vec3<f32>,
    material: ResolvedMaterial,
) -> EvaluatedLighting {
    let loaded_cell = load_light_cache_cell(rng, pixel_id);
    let cell = loaded_cell.cell;
    let exploratory_sample_ratio = loaded_cell.exploratory_sample_ratio;
    let exposure = view.exposure;

    let cell_selected_light = select_light_cell(rng, cell, world_position, world_normal, wo, material);
    let exposure_weighted_cell = log2(cell_selected_light.linear_target_function * exposure + 1.0);
    let cell_confidence = smoothstep(LIGHT_CACHE_CELL_CONFIDENCE_LUM_MIN, LIGHT_CACHE_CELL_CONFIDENCE_LUM_MAX, exposure_weighted_cell);

    // Sample more random lights if our cell has bad lights
    let random_sample_count = u32(round(mix(f32(LIGHT_CACHE_NEW_LIGHTS_SEARCH_COUNT_MAX), f32(LIGHT_CACHE_NEW_LIGHTS_SEARCH_COUNT_MIN), cell_confidence)));
    let random_selected_light = select_light_random(rng, cell, world_position, world_normal, wo, material, random_sample_count);

    let cell_weight = cell_selected_light.weight_sum;
    let random_weight = min(mix(random_selected_light.weight_sum, exploratory_sample_ratio * cell_weight, cell_confidence), random_selected_light.weight_sum);
    let weight_sum = cell_weight + random_weight;

    if weight_sum < 0.0001 {
        return EvaluatedLighting(LightSample(0, 0), vec3(0.0), 0.0, vec3(0.0), false);
    }

    var sel = cell_selected_light.light;
    var sel_weight = cell_selected_light.weight;
    var ucw = weight_sum * cell_selected_light.inverse_target_function;
    if rand_f(rng) < random_weight / weight_sum {
        sel = random_selected_light.light;
        sel_weight = random_selected_light.weight;
        ucw = weight_sum * random_selected_light.inverse_target_function;
    }

    let light_sample = LightSample(sel, rand_u(rng));
    let resolved_light_sample = resolve_light_sample(light_sample);
    let direct_lighting = calculate_resolved_light_contribution(resolved_light_sample, world_position, world_normal);
    let brdf = evaluate_brdf(world_normal, wo, direct_lighting.wi, material);
    let visibility = trace_light_visibility(world_position, resolved_light_sample.world_position);
    let radiance = direct_lighting.radiance;
    let inverse_pdf = ucw * visibility;
    let data = WeightedLight(sel, sel_weight * visibility);
    write_light_cache_cell(pixel_id, local_index, data);

    return EvaluatedLighting(light_sample, radiance, inverse_pdf, direct_lighting.wi, direct_lighting.brdf_rays_can_hit);
}

struct SelectedLight {
    light: u32,
    weight: f32,
    inverse_target_function: f32,
    linear_target_function: f32,
    weight_sum: f32,
}

fn select_light_cell(
    rng: ptr<function, u32>, 
    cell: LightCacheCell,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    wo: vec3<f32>,
    material: ResolvedMaterial,
) -> SelectedLight {
    var p = rand_f(rng);

    let mis_weight = 1.0 / f32(cell.visible_light_count);
    var selected = 0u;
    var selected_target_function = 0.0;
    var selected_linear_target_function = 0.0;
    var selected_weight = 0.0;
    var weight_sum = 0.0;
    // WRS to select the light based on unshadowed contribution
    for (var i = 0u; i < cell.visible_light_count; i++) {
        let light_id = cell.visible_lights[i].light;
        let light_contribution = resolve_and_calculate_light_contribution(LightSample(light_id, rand_u(rng)), world_position, world_normal);
        let brdf = evaluate_brdf(world_normal, wo, light_contribution.wi, material);
        let linear_target_function = luminance(light_contribution.radiance * brdf);
        let target_function = log2(linear_target_function + 1.0);

        let weight = mis_weight * target_function * light_contribution.inverse_pdf;
        weight_sum += weight;

        let prob = weight / weight_sum;
        if p < prob {
            selected = light_id;
            selected_target_function = target_function;
            selected_linear_target_function = linear_target_function;
            selected_weight = weight;
            p /= prob;
        } else {
            p = (p - prob) / (1.0 - prob);
        }
    }

    let inverse_target_function = select(0.0, 1.0 / selected_target_function, selected_target_function > 0.0);
    return SelectedLight(selected, selected_weight, inverse_target_function, selected_linear_target_function, weight_sum);
}

fn select_light_random(
    rng: ptr<function, u32>, 
    cell: LightCacheCell,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    wo: vec3<f32>,
    material: ResolvedMaterial,
    samples: u32,
) -> SelectedLight {
    let light_tile_start = subgroupBroadcastFirst(rand_range_u(128u, rng) * 1024u);
    var p = rand_f(rng);

    let mis_weight = 1.0 / f32(samples);
    var selected = 0u;
    var selected_target_function = 0.0;
    var selected_linear_target_function = 0.0;
    var selected_weight = 0.0;
    var weight_sum = 0.0;
    for (var i = 0u; i < samples; i++) {
        let index = light_tile_start + rand_range_u(1024u, rng);
        let tile_sample = light_tile_resolved_samples[index];
        let light_id = light_tile_samples[index].light_id;

        let sample = unpack_resolved_light_sample(tile_sample);
        let light_contribution = calculate_resolved_light_contribution(sample, world_position, world_normal);
        let brdf = evaluate_brdf(world_normal, wo, light_contribution.wi, material);
        let linear_target_function = luminance(light_contribution.radiance * brdf);
        let target_function = log2(linear_target_function + 1.0);

        let weight = mis_weight * target_function * light_contribution.inverse_pdf;
        weight_sum += weight;

        let prob = weight / weight_sum;
        if p < prob {
            selected = light_id;
            selected_target_function = target_function;
            selected_linear_target_function = linear_target_function;
            selected_weight = weight;
            p /= prob;
        } else {
            p = (p - prob) / (1.0 - prob);
        }
    }

    let inverse_target_function = select(0.0, 1.0 / selected_target_function, selected_target_function > 0.0);
    return SelectedLight(selected, selected_weight, inverse_target_function, selected_linear_target_function, weight_sum);
}
