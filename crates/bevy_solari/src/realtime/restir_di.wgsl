// https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf
// https://d1qx31qr3h6wln.cloudfront.net/publications/ReSTIR%20GI.pdf

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_pbr::utils::{rand_f, rand_range_u, sample_disk}
#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_solari::brdf::evaluate_brdf
#import bevy_solari::gbuffer_utils::{gpixel_resolve, pixel_dissimilar, permute_pixel}
#import bevy_solari::realtime_bindings::{view_output, di_reservoirs_a, di_reservoirs_b, gbuffer, depth_buffer, motion_vectors, previous_gbuffer, previous_depth_buffer, view, previous_view, constants}
#import bevy_solari::presample_light_tiles::{ResolvedLightSamplePacked, unpack_resolved_light_sample}
#import bevy_solari::sampling::{light_contribution_no_trace, resolve_light_sample, trace_light_visibility, balance_heuristic, LightSample}
#import bevy_solari::scene_bindings::{previous_frame_light_id_translations, ResolvedMaterial, LIGHT_NOT_PRESENT_THIS_FRAME}
#import bevy_solari::world_cache::{query_world_cache_lights, evaluate_lighting_from_cache, write_world_cache_light, WORLD_CACHE_CELL_LIFETIME}

const SPATIAL_REUSE_RADIUS_PIXELS = 30.0;
const CONFIDENCE_WEIGHT_CAP = 20.0;

const NULL_RESERVOIR_SAMPLE = 0xFFFFFFFFu;

#define NO_RESTIR

@compute @workgroup_size(8, 8, 1)
fn initial_and_temporal(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.main_pass_viewport.zw)) { return; }

#ifdef NO_RESTIR
    store_reservoir_b(global_id.xy, empty_reservoir());
#else
    let pixel_index = global_id.x + global_id.y * u32(view.main_pass_viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        store_reservoir_b(global_id.xy, empty_reservoir());
        return;
    }
    let surface = gpixel_resolve(textureLoad(gbuffer, global_id.xy, 0), depth, global_id.xy, view.main_pass_viewport.zw, view.world_from_clip);
    let wo = normalize(view.world_position - surface.world_position);

    let initial_reservoir = generate_initial_reservoir(surface.world_position, surface.world_normal, wo, surface.material, &rng);
    let temporal = load_temporal_reservoir(global_id.xy, depth, surface.world_position, surface.world_normal);
    let merge_result = merge_reservoirs(initial_reservoir, surface.world_position, surface.world_normal, surface.material,
        temporal.reservoir, temporal.world_position, temporal.world_normal, temporal.material, &rng);

    store_reservoir_b(global_id.xy, merge_result.merged_reservoir);
#endif
}

@compute @workgroup_size(8, 8, 1)
fn spatial_and_shade(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.main_pass_viewport.zw)) { return; }
    let pixel_index = global_id.x + global_id.y * u32(view.main_pass_viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        store_reservoir_a(global_id.xy, empty_reservoir());
        return;
    }
    let surface = gpixel_resolve(textureLoad(gbuffer, global_id.xy, 0), depth, global_id.xy, view.main_pass_viewport.zw, view.world_from_clip);
    let wo = normalize(view.world_position - surface.world_position);


#ifdef NO_RESTIR
    store_reservoir_a(global_id.xy, empty_reservoir());    
    
    let cell = query_world_cache_lights(&rng, surface.world_position, surface.world_normal, view.world_position);
    let direct_lighting = evaluate_lighting_from_cache(&rng, cell, surface.world_position, surface.world_normal, wo, surface.material, view.exposure);
    write_world_cache_light(&rng, direct_lighting, surface.world_position, surface.world_normal, view.world_position, WORLD_CACHE_CELL_LIFETIME, view.exposure);

    let pixel_color = (direct_lighting.radiance * direct_lighting.inverse_pdf + surface.material.emissive) * view.exposure;
    textureStore(view_output, global_id.xy, vec4(pixel_color, 1.0));
#else
    let input_reservoir = load_reservoir_b(global_id.xy);
    let spatial = load_spatial_reservoir(global_id.xy, depth, surface.world_position, surface.world_normal, &rng);
    let merge_result = merge_reservoirs(input_reservoir, surface.world_position, surface.world_normal, surface.material,
        spatial.reservoir, spatial.world_position, spatial.world_normal, spatial.material, &rng);
    var combined_reservoir = merge_result.merged_reservoir;

    // More accuracy, less stability
#ifndef BIASED_RESAMPLING
    store_reservoir_a(global_id.xy, combined_reservoir);
#endif

    if reservoir_valid(combined_reservoir) {
        let resolved_light_sample = resolve_light_sample(combined_reservoir.sample);
        combined_reservoir.unbiased_contribution_weight *= trace_light_visibility(surface.world_position, resolved_light_sample.world_position);
    }

    // More stability, less accuracy (shadows extend further out than they should)
#ifdef BIASED_RESAMPLING
    store_reservoir_a(global_id.xy, combined_reservoir);
#endif

    let brdf = evaluate_brdf(surface.world_normal, wo, merge_result.wi, surface.material);

    var pixel_color = merge_result.selected_sample_radiance * combined_reservoir.unbiased_contribution_weight;
    pixel_color *= brdf;
    pixel_color += surface.material.emissive;
    pixel_color *= view.exposure;
    textureStore(view_output, global_id.xy, vec4(pixel_color, 1.0));
#endif
}

fn generate_initial_reservoir(world_position: vec3<f32>, world_normal: vec3<f32>, wo: vec3<f32>, material: ResolvedMaterial, rng: ptr<function, u32>) -> Reservoir {
    let cell = query_world_cache_lights(rng, world_position, world_normal, view.world_position);
    let lighting = evaluate_lighting_from_cache(rng, cell, world_position, world_normal, wo, material, view.exposure);
    write_world_cache_light(rng, lighting, world_position, world_normal, view.world_position, WORLD_CACHE_CELL_LIFETIME, view.exposure);
    return Reservoir(lighting.light_sample, 1.0, lighting.inverse_pdf);
}

fn load_temporal_reservoir(pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>) -> NeighborInfo {
    let motion_vector = textureLoad(motion_vectors, pixel_id, 0).xy;
    let temporal_pixel_id_float = round(vec2<f32>(pixel_id) - (motion_vector * view.main_pass_viewport.zw));

    let empty_material = ResolvedMaterial(vec3(0.0), vec3(0.0), vec3(0.0), 1.0, 1.0, 0.0);
    // Check if the current pixel was off screen during the previous frame (current pixel is newly visible),
    // or if all temporal history should assumed to be invalid
    if any(temporal_pixel_id_float < vec2(0.0)) || any(temporal_pixel_id_float >= view.main_pass_viewport.zw) || bool(constants.reset) {
        return NeighborInfo(empty_reservoir(), vec3(0.0), vec3(0.0), empty_material);
    }

    let permuted_temporal_pixel_id = permute_pixel(vec2<u32>(temporal_pixel_id_float), constants.frame_index, view.viewport.zw);
    var temporal = load_temporal_reservoir_inner(permuted_temporal_pixel_id, depth, world_position, world_normal);

    // If permuted reprojection failed (tends to happen on object edges), try point reprojection
    if !reservoir_valid(temporal.reservoir) {
        temporal = load_temporal_reservoir_inner(vec2<u32>(temporal_pixel_id_float), depth, world_position, world_normal);
    }

    // Check if the light selected in the previous frame no longer exists in the current frame (e.g. entity despawned)
    let previous_light_id = temporal.reservoir.sample.light_id >> 16u;
    let triangle_id = temporal.reservoir.sample.light_id & 0xFFFFu;
    let light_id = previous_frame_light_id_translations[previous_light_id];
    if light_id == LIGHT_NOT_PRESENT_THIS_FRAME {
        return NeighborInfo(empty_reservoir(), vec3(0.0), vec3(0.0), empty_material);
    }
    temporal.reservoir.sample.light_id = (light_id << 16u) | triangle_id;

    temporal.reservoir.confidence_weight = min(temporal.reservoir.confidence_weight, CONFIDENCE_WEIGHT_CAP);

    return temporal;
}

fn load_temporal_reservoir_inner(temporal_pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>) -> NeighborInfo {
    // Check if the pixel features have changed heavily between the current and previous frame
    let temporal_depth = textureLoad(previous_depth_buffer, temporal_pixel_id, 0);
    let temporal_surface = gpixel_resolve(textureLoad(previous_gbuffer, temporal_pixel_id, 0), temporal_depth, temporal_pixel_id, view.main_pass_viewport.zw, previous_view.world_from_clip);
    if pixel_dissimilar(depth, world_position, temporal_surface.world_position, world_normal, temporal_surface.world_normal, view) {
        let empty_material = ResolvedMaterial(vec3(0.0), vec3(0.0), vec3(0.0), 1.0, 1.0, 0.0);
        return NeighborInfo(empty_reservoir(), vec3(0.0), vec3(0.0), empty_material);
    }

    let temporal_reservoir = load_reservoir_a(temporal_pixel_id);
    return NeighborInfo(temporal_reservoir, temporal_surface.world_position, temporal_surface.world_normal, temporal_surface.material);
}

fn load_spatial_reservoir(pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>, rng: ptr<function, u32>) -> NeighborInfo {
    for (var i = 0u; i < 5u; i++) {
        let spatial_pixel_id = get_neighbor_pixel_id(pixel_id, rng);

        let spatial_depth = textureLoad(depth_buffer, spatial_pixel_id, 0);
        let spatial_surface = gpixel_resolve(textureLoad(gbuffer, spatial_pixel_id, 0), spatial_depth, spatial_pixel_id, view.main_pass_viewport.zw, view.world_from_clip);
        if pixel_dissimilar(depth, world_position, spatial_surface.world_position, world_normal, spatial_surface.world_normal, view) {
            continue;
        }

        let spatial_reservoir = load_reservoir_b(spatial_pixel_id);
        return NeighborInfo(spatial_reservoir, spatial_surface.world_position, spatial_surface.world_normal, spatial_surface.material);
    }

    let empty_material = ResolvedMaterial(vec3(0.0), vec3(0.0), vec3(0.0), 1.0, 1.0, 0.0);
    return NeighborInfo(empty_reservoir(), world_position, world_normal, empty_material);
}

fn get_neighbor_pixel_id(center_pixel_id: vec2<u32>, rng: ptr<function, u32>) -> vec2<u32> {
    var spatial_id = vec2<f32>(center_pixel_id) + sample_disk(SPATIAL_REUSE_RADIUS_PIXELS, rng);
    spatial_id = clamp(spatial_id, vec2(0.0), view.main_pass_viewport.zw - 1.0);
    return vec2<u32>(spatial_id);
}

struct NeighborInfo {
    reservoir: Reservoir,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    material: ResolvedMaterial,
}

struct Reservoir {
    sample: LightSample,
    confidence_weight: f32,
    unbiased_contribution_weight: f32,
}

fn empty_reservoir() -> Reservoir {
    return Reservoir(
        LightSample(NULL_RESERVOIR_SAMPLE, 0u),
        0.0,
        0.0,
    );
}

fn reservoir_valid(reservoir: Reservoir) -> bool {
    return reservoir.sample.light_id != NULL_RESERVOIR_SAMPLE;
}

fn pack_reservoir(reservoir: Reservoir) -> vec4<u32> {
    let weights = bitcast<vec2<u32>>(vec2<f32>(reservoir.confidence_weight, reservoir.unbiased_contribution_weight));
    return vec4<u32>(reservoir.sample.light_id, reservoir.sample.seed, weights);
}

fn store_reservoir_a(pixel: vec2<u32>, reservoir: Reservoir) {
    textureStore(di_reservoirs_a, pixel, pack_reservoir(reservoir));
}

fn store_reservoir_b(pixel: vec2<u32>, reservoir: Reservoir) {
    textureStore(di_reservoirs_b, pixel, pack_reservoir(reservoir));
}

fn unpack_reservoir(packed: vec4<u32>) -> Reservoir {
    let weights = bitcast<vec2<f32>>(packed.zw);
    return Reservoir(LightSample(packed.x, packed.y), weights.x, weights.y);
}

fn load_reservoir_a(pixel: vec2<u32>) -> Reservoir {
    return unpack_reservoir(textureLoad(di_reservoirs_a, pixel));
}

fn load_reservoir_b(pixel: vec2<u32>) -> Reservoir {
    return unpack_reservoir(textureLoad(di_reservoirs_b, pixel));
}

struct ReservoirMergeResult {
    merged_reservoir: Reservoir,
    selected_sample_radiance: vec3<f32>,
    wi: vec3<f32>,
}

fn merge_reservoirs(
    canonical_reservoir: Reservoir,
    canonical_world_position: vec3<f32>,
    canonical_world_normal: vec3<f32>,
    canonical_material: ResolvedMaterial,
    other_reservoir: Reservoir,
    other_world_position: vec3<f32>,
    other_world_normal: vec3<f32>,
    other_material: ResolvedMaterial,
    rng: ptr<function, u32>,
) -> ReservoirMergeResult {
    // Contributions for resampling
    let canonical_contribution_canonical_sample = reservoir_contribution(canonical_reservoir, canonical_world_position, canonical_world_normal, canonical_material);
    let canonical_contribution_other_sample = reservoir_contribution(other_reservoir, canonical_world_position, canonical_world_normal, canonical_material);

    // Extra contributions for MIS
    let other_contribution_canonical_sample = reservoir_contribution(canonical_reservoir, other_world_position, other_world_normal, other_material);
    let other_contribution_other_sample = reservoir_contribution(other_reservoir, other_world_position, other_world_normal, other_material);

    // Resampling weight for canonical sample
    let canonical_sample_mis_weight = balance_heuristic(
        canonical_reservoir.confidence_weight * canonical_contribution_canonical_sample.target_function,
        other_reservoir.confidence_weight * other_contribution_canonical_sample.target_function,
    );
    let canonical_sample_resampling_weight = canonical_sample_mis_weight * canonical_contribution_canonical_sample.target_function * canonical_reservoir.unbiased_contribution_weight;

    // Resampling weight for other sample
    let other_sample_mis_weight = balance_heuristic(
        other_reservoir.confidence_weight * other_contribution_other_sample.target_function,
        canonical_reservoir.confidence_weight * canonical_contribution_other_sample.target_function,
    );
    let other_sample_resampling_weight = other_sample_mis_weight * canonical_contribution_other_sample.target_function * other_reservoir.unbiased_contribution_weight;

    // Perform resampling
    var combined_reservoir = empty_reservoir();
    combined_reservoir.confidence_weight = canonical_reservoir.confidence_weight + other_reservoir.confidence_weight;
    let weight_sum = canonical_sample_resampling_weight + other_sample_resampling_weight;

    if rand_f(rng) < other_sample_resampling_weight / weight_sum {
        combined_reservoir.sample = other_reservoir.sample;

        let inverse_target_function = select(0.0, 1.0 / canonical_contribution_other_sample.target_function, canonical_contribution_other_sample.target_function > 0.0);
        combined_reservoir.unbiased_contribution_weight = weight_sum * inverse_target_function;

        return ReservoirMergeResult(combined_reservoir, canonical_contribution_other_sample.radiance, canonical_contribution_other_sample.wi);
    } else {
        combined_reservoir.sample = canonical_reservoir.sample;

        let inverse_target_function = select(0.0, 1.0 / canonical_contribution_canonical_sample.target_function, canonical_contribution_canonical_sample.target_function > 0.0);
        combined_reservoir.unbiased_contribution_weight = weight_sum * inverse_target_function;

        return ReservoirMergeResult(combined_reservoir, canonical_contribution_canonical_sample.radiance, canonical_contribution_canonical_sample.wi);
    }
}

struct ReservoirContribution {
    radiance: vec3<f32>,
    target_function: f32,
    wi: vec3<f32>,
}

// TODO: Have input take ResolvedLightSample instead of reservoir.light_sample
fn reservoir_contribution(reservoir: Reservoir, world_position: vec3<f32>, world_normal: vec3<f32>, material: ResolvedMaterial) -> ReservoirContribution {
    if !reservoir_valid(reservoir) { return ReservoirContribution(vec3(0.0), 0.0, vec3(0.0)); }
    let light_contribution = light_contribution_no_trace(reservoir.sample, world_position, world_normal);
    let wo = normalize(view.world_position - world_position);
    let brdf = evaluate_brdf(world_normal, wo, light_contribution.wi, material);
    let target_function = luminance(light_contribution.radiance * brdf);
    return ReservoirContribution(light_contribution.radiance, target_function, light_contribution.wi);
}
