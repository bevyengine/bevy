// https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_pbr::utils::{rand_f, sample_uniform_hemisphere, uniform_hemisphere_inverse_pdf, sample_disk}
#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_solari::brdf::evaluate_diffuse_brdf
#import bevy_solari::gbuffer_utils::{gpixel_resolve, pixel_dissimilar, permute_pixel}
#import bevy_solari::sampling::{sample_random_light, trace_point_visibility, balance_heuristic}
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, RAY_T_MIN, RAY_T_MAX}
#import bevy_solari::world_cache::{query_world_cache, WORLD_CACHE_CELL_LIFETIME}

@group(1) @binding(0) var view_output: texture_storage_2d<rgba16float, read_write>;
@group(1) @binding(5) var<storage, read_write> gi_reservoirs_a: array<Reservoir>;
@group(1) @binding(6) var<storage, read_write> gi_reservoirs_b: array<Reservoir>;
@group(1) @binding(7) var gbuffer: texture_2d<u32>;
@group(1) @binding(8) var depth_buffer: texture_depth_2d;
@group(1) @binding(9) var motion_vectors: texture_2d<f32>;
@group(1) @binding(10) var previous_gbuffer: texture_2d<u32>;
@group(1) @binding(11) var previous_depth_buffer: texture_depth_2d;
@group(1) @binding(12) var<uniform> view: View;
@group(1) @binding(13) var<uniform> previous_view: PreviousViewUniforms;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

const SPATIAL_REUSE_RADIUS_PIXELS = 30.0;
const CONFIDENCE_WEIGHT_CAP = 8.0;

@compute @workgroup_size(8, 8, 1)
fn initial_and_temporal(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.main_pass_viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.main_pass_viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        gi_reservoirs_b[pixel_index] = empty_reservoir();
        return;
    }
    let surface = gpixel_resolve(textureLoad(gbuffer, global_id.xy, 0), depth, global_id.xy, view.main_pass_viewport.zw, view.world_from_clip);

    let initial_reservoir = generate_initial_reservoir(surface.world_position, surface.world_normal, &rng);
    let temporal = load_temporal_reservoir(global_id.xy, depth, surface.world_position, surface.world_normal);
    let merge_result = merge_reservoirs(initial_reservoir, surface.world_position, surface.world_normal, surface.material.base_color / PI,
        temporal.reservoir, temporal.world_position, temporal.world_normal, temporal.diffuse_brdf, &rng);

    gi_reservoirs_b[pixel_index] = merge_result.merged_reservoir;
}

@compute @workgroup_size(8, 8, 1)
fn spatial_and_shade(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.main_pass_viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.main_pass_viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        gi_reservoirs_a[pixel_index] = empty_reservoir();
        return;
    }
    let surface = gpixel_resolve(textureLoad(gbuffer, global_id.xy, 0), depth, global_id.xy, view.main_pass_viewport.zw, view.world_from_clip);

    let input_reservoir = gi_reservoirs_b[pixel_index];
    let spatial = load_spatial_reservoir(global_id.xy, depth, surface.world_position, surface.world_normal, &rng);
    let merge_result = merge_reservoirs(input_reservoir, surface.world_position, surface.world_normal, surface.material.base_color / PI,
        spatial.reservoir, spatial.world_position, spatial.world_normal, spatial.diffuse_brdf, &rng);
    var combined_reservoir = merge_result.merged_reservoir;

    // More accuracy, less stability
#ifndef BIASED_RESAMPLING
    gi_reservoirs_a[pixel_index] = combined_reservoir;
#endif

    combined_reservoir.unbiased_contribution_weight *= trace_point_visibility(surface.world_position, combined_reservoir.sample_point_world_position);

    // More stability, less accuracy (shadows extend further out than they should)
#ifdef BIASED_RESAMPLING
    gi_reservoirs_a[pixel_index] = combined_reservoir;
#endif

    let brdf = evaluate_diffuse_brdf(surface.material.base_color, surface.material.metallic);

    var pixel_color = textureLoad(view_output, global_id.xy);
    pixel_color += vec4(merge_result.selected_sample_radiance * combined_reservoir.unbiased_contribution_weight * view.exposure * brdf, 0.0);
    textureStore(view_output, global_id.xy, pixel_color);
}

fn generate_initial_reservoir(world_position: vec3<f32>, world_normal: vec3<f32>, rng: ptr<function, u32>) -> Reservoir {
    var reservoir = empty_reservoir();

    let ray_direction = sample_uniform_hemisphere(world_normal, rng);
    let ray_hit = trace_ray(world_position, ray_direction, RAY_T_MIN, RAY_T_MAX, RAY_FLAG_NONE);

    if ray_hit.kind == RAY_QUERY_INTERSECTION_NONE {
        return reservoir;
    }

    let sample_point = resolve_ray_hit_full(ray_hit);

    if all(sample_point.material.emissive != vec3(0.0)) {
        return reservoir;
    }

    reservoir.sample_point_world_position = sample_point.world_position;
    reservoir.sample_point_world_normal = sample_point.world_normal;
    reservoir.confidence_weight = 1.0;

#ifdef NO_WORLD_CACHE
    let direct_lighting = sample_random_light(sample_point.world_position, sample_point.world_normal, rng);
    reservoir.radiance = direct_lighting.radiance;
    reservoir.unbiased_contribution_weight = direct_lighting.inverse_pdf * uniform_hemisphere_inverse_pdf();
#else
    reservoir.radiance = query_world_cache(sample_point.world_position, sample_point.geometric_world_normal, view.world_position, ray.t, WORLD_CACHE_CELL_LIFETIME, rng);
    reservoir.unbiased_contribution_weight = uniform_hemisphere_inverse_pdf();
#endif

    let sample_point_diffuse_brdf = sample_point.material.base_color / PI;
    reservoir.radiance *= sample_point_diffuse_brdf;

    return reservoir;
}

fn load_temporal_reservoir(pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>) -> NeighborInfo {
    let motion_vector = textureLoad(motion_vectors, pixel_id, 0).xy;
    let temporal_pixel_id_float = round(vec2<f32>(pixel_id) - (motion_vector * view.main_pass_viewport.zw));

    // Check if the current pixel was off screen during the previous frame (current pixel is newly visible),
    // or if all temporal history should assumed to be invalid
    if any(temporal_pixel_id_float < vec2(0.0)) || any(temporal_pixel_id_float >= view.main_pass_viewport.zw) || bool(constants.reset) {
        return NeighborInfo(empty_reservoir(), vec3(0.0), vec3(0.0), vec3(0.0));
    }

    let permuted_temporal_pixel_id = permute_pixel(vec2<u32>(temporal_pixel_id_float), constants.frame_index, view.main_pass_viewport.zw);
    var temporal = load_temporal_reservoir_inner(permuted_temporal_pixel_id, depth, world_position, world_normal);

    // If permuted reprojection failed (tends to happen on object edges), try point reprojection
    if all(temporal.reservoir.radiance == vec3(0.0)) {
        temporal = load_temporal_reservoir_inner(vec2<u32>(temporal_pixel_id_float), depth, world_position, world_normal);
    }

    temporal.reservoir.confidence_weight = min(temporal.reservoir.confidence_weight, CONFIDENCE_WEIGHT_CAP);

    return temporal;
}

fn load_temporal_reservoir_inner(temporal_pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>) -> NeighborInfo {
    // Check if the pixel features have changed heavily between the current and previous frame
    let temporal_depth = textureLoad(previous_depth_buffer, temporal_pixel_id, 0);
    let temporal_surface = gpixel_resolve(textureLoad(previous_gbuffer, temporal_pixel_id, 0), temporal_depth, temporal_pixel_id, view.main_pass_viewport.zw, previous_view.world_from_clip);
    let temporal_diffuse_brdf = temporal_surface.material.base_color / PI;
    if pixel_dissimilar(depth, world_position, temporal_surface.world_position, world_normal, temporal_surface.world_normal, view) {
        return NeighborInfo(empty_reservoir(), vec3(0.0), vec3(0.0), vec3(0.0));
    }

    let temporal_pixel_index = temporal_pixel_id.x + temporal_pixel_id.y * u32(view.main_pass_viewport.z);
    let temporal_reservoir = gi_reservoirs_a[temporal_pixel_index];

    return NeighborInfo(temporal_reservoir, temporal_surface.world_position, temporal_surface.world_normal, temporal_diffuse_brdf);
}

fn load_spatial_reservoir(pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>, rng: ptr<function, u32>) -> NeighborInfo {
    for (var i = 0u; i < 5u; i++) {
        let spatial_pixel_id = get_neighbor_pixel_id(pixel_id, rng);

        let spatial_depth = textureLoad(depth_buffer, spatial_pixel_id, 0);
        let spatial_surface = gpixel_resolve(textureLoad(gbuffer, spatial_pixel_id, 0), spatial_depth, spatial_pixel_id, view.main_pass_viewport.zw, view.world_from_clip);
        let spatial_diffuse_brdf = spatial_surface.material.base_color / PI;
        if pixel_dissimilar(depth, world_position, spatial_surface.world_position, world_normal, spatial_surface.world_normal, view) {
            continue;
        }

        let spatial_pixel_index = spatial_pixel_id.x + spatial_pixel_id.y * u32(view.main_pass_viewport.z);
        let spatial_reservoir = gi_reservoirs_b[spatial_pixel_index];
        return NeighborInfo(spatial_reservoir, spatial_surface.world_position, spatial_surface.world_normal, spatial_diffuse_brdf);
    }

    return NeighborInfo(empty_reservoir(), world_position, world_normal, vec3(0.0));
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
    diffuse_brdf: vec3<f32>,
}

fn jacobian(
    new_world_position: vec3<f32>,
    original_world_position: vec3<f32>,
    sample_point_world_position: vec3<f32>,
    sample_point_world_normal: vec3<f32>,
) -> f32 {
    let r = new_world_position - sample_point_world_position;
    let q = original_world_position - sample_point_world_position;
    let rl = length(r);
    let ql = length(q);
    let phi_r = saturate(dot(r / rl, sample_point_world_normal));
    let phi_q = saturate(dot(q / ql, sample_point_world_normal));
    let jacobian = (phi_r * ql * ql) / (phi_q * rl * rl);
    return select(jacobian, 0.0, isinf(jacobian) || isnan(jacobian));
}

fn isinf(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7fffffffu) == 0x7f800000u;
}

fn isnan(x: f32) -> bool {
    return (bitcast<u32>(x) & 0x7fffffffu) > 0x7f800000u;
}

// Don't adjust the size of this struct without also adjusting GI_RESERVOIR_STRUCT_SIZE.
struct Reservoir {
    sample_point_world_position: vec3<f32>,
    weight_sum: f32,
    radiance: vec3<f32>,
    confidence_weight: f32,
    sample_point_world_normal: vec3<f32>,
    unbiased_contribution_weight: f32,
}

fn empty_reservoir() -> Reservoir {
    return Reservoir(
        vec3(0.0),
        0.0,
        vec3(0.0),
        0.0,
        vec3(0.0),
        0.0,
    );
}

struct ReservoirMergeResult {
    merged_reservoir: Reservoir,
    selected_sample_radiance: vec3<f32>,
}

fn merge_reservoirs(
    canonical_reservoir: Reservoir,
    canonical_world_position: vec3<f32>,
    canonical_world_normal: vec3<f32>,
    canonical_diffuse_brdf: vec3<f32>,
    other_reservoir: Reservoir,
    other_world_position: vec3<f32>,
    other_world_normal: vec3<f32>,
    other_diffuse_brdf: vec3<f32>,
    rng: ptr<function, u32>,
) -> ReservoirMergeResult {
    // Radiances for resampling
    let canonical_sample_radiance = canonical_reservoir.radiance * saturate(dot(normalize(canonical_reservoir.sample_point_world_position - canonical_world_position), canonical_world_normal));
    let other_sample_radiance = other_reservoir.radiance * saturate(dot(normalize(other_reservoir.sample_point_world_position - canonical_world_position), canonical_world_normal));

    // Target functions for resampling and MIS
    let canonical_target_function_canonical_sample = luminance(canonical_sample_radiance * canonical_diffuse_brdf);
    let canonical_target_function_other_sample = luminance(other_sample_radiance * canonical_diffuse_brdf);

    // Extra target functions for MIS
    let other_target_function_canonical_sample = luminance(
        canonical_reservoir.radiance * saturate(dot(normalize(canonical_reservoir.sample_point_world_position - other_world_position), other_world_normal)) * other_diffuse_brdf
    );
    let other_target_function_other_sample = luminance(
        other_reservoir.radiance * saturate(dot(normalize(other_reservoir.sample_point_world_position - other_world_position), other_world_normal)) * other_diffuse_brdf
    );

    // Jacobians for resampling and MIS
    let canonical_target_function_other_sample_jacobian = jacobian(
        canonical_world_position,
        other_world_position,
        other_reservoir.sample_point_world_position,
        other_reservoir.sample_point_world_normal
    );
    let other_target_function_canonical_sample_jacobian = jacobian(
        other_world_position,
        canonical_world_position,
        canonical_reservoir.sample_point_world_position,
        canonical_reservoir.sample_point_world_normal
    );

    // Don't merge samples with huge jacobians, as it explodes the variance
    if canonical_target_function_other_sample_jacobian > 1.2 {
        return ReservoirMergeResult(canonical_reservoir, canonical_sample_radiance);
    }

    // Resampling weight for canonical sample
    let canonical_sample_mis_weight = balance_heuristic(
        canonical_reservoir.confidence_weight * canonical_target_function_canonical_sample,
        other_reservoir.confidence_weight * other_target_function_canonical_sample * other_target_function_canonical_sample_jacobian,
    );
    let canonical_sample_resampling_weight = canonical_sample_mis_weight * canonical_target_function_canonical_sample * canonical_reservoir.unbiased_contribution_weight;

    // Resampling weight for other sample
    let other_sample_mis_weight = balance_heuristic(
        other_reservoir.confidence_weight * other_target_function_other_sample,
        canonical_reservoir.confidence_weight * canonical_target_function_other_sample * canonical_target_function_other_sample_jacobian,
    );
    let other_sample_resampling_weight = other_sample_mis_weight * canonical_target_function_other_sample * other_reservoir.unbiased_contribution_weight * canonical_target_function_other_sample_jacobian;

    // Perform resampling
    var combined_reservoir = empty_reservoir();
    combined_reservoir.confidence_weight = canonical_reservoir.confidence_weight + other_reservoir.confidence_weight;
    combined_reservoir.weight_sum = canonical_sample_resampling_weight + other_sample_resampling_weight;

    if rand_f(rng) < other_sample_resampling_weight / combined_reservoir.weight_sum {
        combined_reservoir.sample_point_world_position = other_reservoir.sample_point_world_position;
        combined_reservoir.sample_point_world_normal = other_reservoir.sample_point_world_normal;
        combined_reservoir.radiance = other_reservoir.radiance;

        let inverse_target_function = select(0.0, 1.0 / canonical_target_function_other_sample, canonical_target_function_other_sample > 0.0);
        combined_reservoir.unbiased_contribution_weight = combined_reservoir.weight_sum * inverse_target_function;

        return ReservoirMergeResult(combined_reservoir, other_sample_radiance);
    } else {
        combined_reservoir.sample_point_world_position = canonical_reservoir.sample_point_world_position;
        combined_reservoir.sample_point_world_normal = canonical_reservoir.sample_point_world_normal;
        combined_reservoir.radiance = canonical_reservoir.radiance;

        let inverse_target_function = select(0.0, 1.0 / canonical_target_function_canonical_sample, canonical_target_function_canonical_sample > 0.0);
        combined_reservoir.unbiased_contribution_weight = combined_reservoir.weight_sum * inverse_target_function;

        return ReservoirMergeResult(combined_reservoir, canonical_sample_radiance);
    }
}
