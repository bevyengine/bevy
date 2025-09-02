// https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::pbr_deferred_types::unpack_24bit_normal
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_pbr::utils::{rand_f, sample_uniform_hemisphere, uniform_hemisphere_inverse_pdf, sample_disk, octahedral_decode}
#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_solari::sampling::{sample_random_light, trace_point_visibility}
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, RAY_T_MIN, RAY_T_MAX}
#import bevy_solari::world_cache::query_world_cache

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
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let base_color = pow(unpack4x8unorm(gpixel.r).rgb, vec3(2.2));
    let diffuse_brdf = base_color / PI;

    let initial_reservoir = generate_initial_reservoir(world_position, world_normal, &rng);
    let temporal = load_temporal_reservoir(global_id.xy, depth, world_position, world_normal);
    let merge_result = merge_reservoirs(initial_reservoir, world_position, world_normal, diffuse_brdf,
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
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let base_color = pow(unpack4x8unorm(gpixel.r).rgb, vec3(2.2));
    let diffuse_brdf = base_color / PI;

    let input_reservoir = gi_reservoirs_b[pixel_index];
    let spatial = load_spatial_reservoir(global_id.xy, depth, world_position, world_normal, &rng);
    let merge_result = merge_reservoirs(input_reservoir, world_position, world_normal, diffuse_brdf,
        spatial.reservoir, spatial.world_position, spatial.world_normal, spatial.diffuse_brdf, &rng);
    let combined_reservoir = merge_result.merged_reservoir;

    gi_reservoirs_a[pixel_index] = combined_reservoir;

    var pixel_color = textureLoad(view_output, global_id.xy);
    pixel_color += vec4(merge_result.selected_sample_radiance * combined_reservoir.unbiased_contribution_weight * view.exposure, 0.0);
    textureStore(view_output, global_id.xy, pixel_color);

#ifdef VISUALIZE_WORLD_CACHE
    textureStore(view_output, global_id.xy, vec4(query_world_cache(world_position, world_normal, view.world_position) * view.exposure, 1.0));
#endif
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
    reservoir.radiance = query_world_cache(sample_point.world_position, sample_point.geometric_world_normal, view.world_position);
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

    let temporal_pixel_id_base = vec2<u32>(round(temporal_pixel_id_float));
    for (var i = 0u; i < 4u; i++) {
        let temporal_pixel_id = permute_pixel(temporal_pixel_id_base, i);

        // Check if the pixel features have changed heavily between the current and previous frame
        let temporal_depth = textureLoad(previous_depth_buffer, temporal_pixel_id, 0);
        let temporal_gpixel = textureLoad(previous_gbuffer, temporal_pixel_id, 0);
        let temporal_world_position = reconstruct_previous_world_position(temporal_pixel_id, temporal_depth);
        let temporal_world_normal = octahedral_decode(unpack_24bit_normal(temporal_gpixel.a));
        let temporal_base_color = pow(unpack4x8unorm(temporal_gpixel.r).rgb, vec3(2.2));
        let temporal_diffuse_brdf = temporal_base_color / PI;
        if pixel_dissimilar(depth, world_position, temporal_world_position, world_normal, temporal_world_normal) {
            continue;
        }

        let temporal_pixel_index = temporal_pixel_id.x + temporal_pixel_id.y * u32(view.main_pass_viewport.z);
        var temporal_reservoir = gi_reservoirs_a[temporal_pixel_index];

        temporal_reservoir.confidence_weight = min(temporal_reservoir.confidence_weight, CONFIDENCE_WEIGHT_CAP);

        return NeighborInfo(temporal_reservoir, temporal_world_position, temporal_world_normal, temporal_diffuse_brdf);
    }

    return NeighborInfo(empty_reservoir(), vec3(0.0), vec3(0.0), vec3(0.0));
}

fn permute_pixel(pixel_id: vec2<u32>, i: u32) -> vec2<u32> {
    let r = constants.frame_index + i;
    let offset = vec2(r & 3u, (r >> 2u) & 3u);
    var shifted_pixel_id = pixel_id + offset;
    shifted_pixel_id ^= vec2(3u);
    shifted_pixel_id -= offset;
    return min(shifted_pixel_id, vec2<u32>(view.main_pass_viewport.zw - 1.0));
}

fn load_spatial_reservoir(pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>, rng: ptr<function, u32>) -> NeighborInfo {
    let spatial_pixel_id = get_neighbor_pixel_id(pixel_id, rng);

    let spatial_depth = textureLoad(depth_buffer, spatial_pixel_id, 0);
    let spatial_gpixel = textureLoad(gbuffer, spatial_pixel_id, 0);
    let spatial_world_position = reconstruct_world_position(spatial_pixel_id, spatial_depth);
    let spatial_world_normal = octahedral_decode(unpack_24bit_normal(spatial_gpixel.a));
    let spatial_base_color = pow(unpack4x8unorm(spatial_gpixel.r).rgb, vec3(2.2));
    let spatial_diffuse_brdf = spatial_base_color / PI;
    if pixel_dissimilar(depth, world_position, spatial_world_position, world_normal, spatial_world_normal) {
        return NeighborInfo(empty_reservoir(), spatial_world_position, spatial_world_normal, spatial_diffuse_brdf);
    }

    let spatial_pixel_index = spatial_pixel_id.x + spatial_pixel_id.y * u32(view.main_pass_viewport.z);
    var spatial_reservoir = gi_reservoirs_b[spatial_pixel_index];

    spatial_reservoir.radiance *= trace_point_visibility(world_position, spatial_reservoir.sample_point_world_position);

    return NeighborInfo(spatial_reservoir, spatial_world_position, spatial_world_normal, spatial_diffuse_brdf);
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

fn reconstruct_world_position(pixel_id: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_id) + 0.5) / view.main_pass_viewport.zw;
    let xy_ndc = (uv - vec2(0.5)) * vec2(2.0, -2.0);
    let world_pos = view.world_from_clip * vec4(xy_ndc, depth, 1.0);
    return world_pos.xyz / world_pos.w;
}

fn reconstruct_previous_world_position(pixel_id: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_id) + 0.5) / view.main_pass_viewport.zw;
    let xy_ndc = (uv - vec2(0.5)) * vec2(2.0, -2.0);
    let world_pos = previous_view.world_from_clip * vec4(xy_ndc, depth, 1.0);
    return world_pos.xyz / world_pos.w;
}

// Reject if tangent plane difference difference more than 0.3% or angle between normals more than 25 degrees
fn pixel_dissimilar(depth: f32, world_position: vec3<f32>, other_world_position: vec3<f32>, normal: vec3<f32>, other_normal: vec3<f32>) -> bool {
    // https://developer.download.nvidia.com/video/gputechconf/gtc/2020/presentations/s22699-fast-denoising-with-self-stabilizing-recurrent-blurs.pdf#page=45
    let tangent_plane_distance = abs(dot(normal, other_world_position - world_position));
    let view_z = -depth_ndc_to_view_z(depth);

    return tangent_plane_distance / view_z > 0.003 || dot(normal, other_normal) < 0.906;
}

fn depth_ndc_to_view_z(ndc_depth: f32) -> f32 {
#ifdef VIEW_PROJECTION_PERSPECTIVE
    return -view.clip_from_view[3][2]() / ndc_depth;
#else ifdef VIEW_PROJECTION_ORTHOGRAPHIC
    return -(view.clip_from_view[3][2] - ndc_depth) / view.clip_from_view[2][2];
#else
    let view_pos = view.view_from_clip * vec4(0.0, 0.0, ndc_depth, 1.0);
    return view_pos.z / view_pos.w;
#endif
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
    let canonical_sample_radiance =
        canonical_reservoir.radiance *
        saturate(dot(normalize(canonical_reservoir.sample_point_world_position - canonical_world_position), canonical_world_normal)) *
        canonical_diffuse_brdf;
    let other_sample_radiance =
        other_reservoir.radiance *
        saturate(dot(normalize(other_reservoir.sample_point_world_position - canonical_world_position), canonical_world_normal)) *
        canonical_diffuse_brdf;

    // Target functions for resampling and MIS
    let canonical_target_function_canonical_sample = luminance(canonical_sample_radiance);
    let canonical_target_function_other_sample = luminance(other_sample_radiance);

    // Extra target functions for MIS
    let other_target_function_canonical_sample = luminance(
        canonical_reservoir.radiance *
        saturate(dot(normalize(canonical_reservoir.sample_point_world_position - other_world_position), other_world_normal)) *
        other_diffuse_brdf
    );
    let other_target_function_other_sample = luminance(
        other_reservoir.radiance *
        saturate(dot(normalize(other_reservoir.sample_point_world_position - other_world_position), other_world_normal)) *
        other_diffuse_brdf
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
    if canonical_target_function_other_sample_jacobian > 2.0 {
        return ReservoirMergeResult(canonical_reservoir, canonical_sample_radiance);
    }

    // Resampling weight for canonical sample
    let canonical_sample_mis_weight = balance_heuristic(
        canonical_reservoir.confidence_weight * canonical_target_function_canonical_sample,
        other_reservoir.confidence_weight * other_target_function_canonical_sample * other_target_function_canonical_sample_jacobian,
    );
    let canonical_sample_resampling_weight = canonical_sample_mis_weight *
        canonical_target_function_canonical_sample *
        canonical_reservoir.unbiased_contribution_weight;

    // Resampling weight for other sample
    let other_sample_mis_weight = balance_heuristic(
        other_reservoir.confidence_weight * other_target_function_other_sample,
        canonical_reservoir.confidence_weight * canonical_target_function_other_sample * canonical_target_function_other_sample_jacobian,
    );
    let other_sample_resampling_weight = other_sample_mis_weight *
        canonical_target_function_other_sample *
        other_reservoir.unbiased_contribution_weight *
        canonical_target_function_other_sample_jacobian;

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

fn balance_heuristic(x: f32, y: f32) -> f32 {
    let sum = x + y;
    if sum == 0.0 {
        return 0.0;
    }
    return x / sum;
}
