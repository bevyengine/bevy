// https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::pbr_deferred_types::unpack_24bit_normal
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_pbr::rgb9e5::rgb9e5_to_vec3_
#import bevy_pbr::utils::{rand_f, octahedral_decode}
#import bevy_render::maths::{PI, PI_2}
#import bevy_render::view::View
#import bevy_solari::sampling::{sample_uniform_hemisphere, sample_random_light}
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, RAY_T_MIN, RAY_T_MAX}

@group(1) @binding(0) var view_output: texture_storage_2d<rgba16float, read_write>;
@group(1) @binding(3) var<storage, read_write> gi_reservoirs_a: array<Reservoir>;
@group(1) @binding(4) var<storage, read_write> gi_reservoirs_b: array<Reservoir>;
@group(1) @binding(5) var gbuffer: texture_2d<u32>;
@group(1) @binding(6) var depth_buffer: texture_depth_2d;
@group(1) @binding(7) var motion_vectors: texture_2d<f32>;
@group(1) @binding(8) var previous_gbuffer: texture_2d<u32>;
@group(1) @binding(9) var previous_depth_buffer: texture_depth_2d;
@group(1) @binding(10) var<uniform> view: View;
@group(1) @binding(11) var<uniform> previous_view: PreviousViewUniforms;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

const CONFIDENCE_WEIGHT_CAP = 30.0;

@compute @workgroup_size(8, 8, 1)
fn initial_and_temporal(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
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

    let temporal_reservoir = load_temporal_reservoir(global_id.xy, depth, world_position, world_normal);

    let ray_direction = sample_uniform_hemisphere(world_normal, &rng);
    let ray_hit = trace_ray(world_position, ray_direction, RAY_T_MIN, RAY_T_MAX, RAY_FLAG_NONE);
    if ray_hit.kind == RAY_QUERY_INTERSECTION_NONE {
        gi_reservoirs_b[pixel_index] = temporal_reservoir;
        return;
    }
    let sample_point = resolve_ray_hit_full(ray_hit);
    if all(sample_point.material.emissive != vec3(0.0)) {
        gi_reservoirs_b[pixel_index] = temporal_reservoir;
        return;
    }
    let sample_point_diffuse_brdf = sample_point.material.base_color / PI;
    let direct_lighting = sample_random_light(sample_point.world_position, sample_point.world_normal, &rng);
    let sample_point_radiance = direct_lighting.radiance * sample_point_diffuse_brdf;

    let cos_theta = dot(ray_direction, world_normal);
    let inverse_uniform_hemisphere_pdf = PI_2;

    var combined_reservoir = empty_reservoir();
    combined_reservoir.confidence_weight = 1.0 + temporal_reservoir.confidence_weight;

    let mis_weight_denominator = 1.0 / combined_reservoir.confidence_weight;

    let new_mis_weight = mis_weight_denominator;
    let new_target_function = luminance(sample_point_radiance * diffuse_brdf * cos_theta);
    let new_inverse_pdf = direct_lighting.inverse_pdf * inverse_uniform_hemisphere_pdf;
    let new_resampling_weight = new_mis_weight * (new_target_function * new_inverse_pdf);

    let temporal_mis_weight = temporal_reservoir.confidence_weight * mis_weight_denominator;
    let temporal_cos_theta = dot(normalize(temporal_reservoir.sample_point_world_position - world_position), world_normal);
    let temporal_target_function = luminance(temporal_reservoir.radiance * diffuse_brdf * temporal_cos_theta);
    let temporal_resampling_weight = temporal_mis_weight * (temporal_target_function * temporal_reservoir.unbiased_contribution_weight);

    combined_reservoir.weight_sum = new_resampling_weight + temporal_resampling_weight;

    if rand_f(&rng) < temporal_resampling_weight / combined_reservoir.weight_sum {
        combined_reservoir.sample_point_world_position = temporal_reservoir.sample_point_world_position;
        combined_reservoir.radiance = temporal_reservoir.radiance;

        let inverse_target_function = select(0.0, 1.0 / temporal_target_function, temporal_target_function > 0.0);
        combined_reservoir.unbiased_contribution_weight = combined_reservoir.weight_sum * inverse_target_function;
    } else {
        combined_reservoir.sample_point_world_position = sample_point.world_position;
        combined_reservoir.radiance = sample_point_radiance;

        let inverse_target_function = select(0.0, 1.0 / new_target_function, new_target_function > 0.0);
        combined_reservoir.unbiased_contribution_weight = combined_reservoir.weight_sum * inverse_target_function;
    }

    gi_reservoirs_b[pixel_index] = combined_reservoir;
}


@compute @workgroup_size(8, 8, 1)
fn spatial_and_shade(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
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
    let cos_theta = dot(normalize(input_reservoir.sample_point_world_position - world_position), world_normal);
    let radiance = input_reservoir.radiance * diffuse_brdf * cos_theta;

    gi_reservoirs_a[pixel_index] = input_reservoir;

    var pixel_color = textureLoad(view_output, global_id.xy);
    pixel_color += vec4(radiance * input_reservoir.unbiased_contribution_weight * view.exposure, 0.0);
    textureStore(view_output, global_id.xy, pixel_color);
}

fn load_temporal_reservoir(pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>) -> Reservoir {
    let motion_vector = textureLoad(motion_vectors, pixel_id, 0).xy;
    let temporal_pixel_id_float = round(vec2<f32>(pixel_id) - (motion_vector * view.viewport.zw));
    let temporal_pixel_id = vec2<u32>(temporal_pixel_id_float);
    if any(temporal_pixel_id_float < vec2(0.0)) || any(temporal_pixel_id_float >= view.viewport.zw) || bool(constants.reset) {
        return empty_reservoir();
    }

    let temporal_depth = textureLoad(previous_depth_buffer, temporal_pixel_id, 0);
    let temporal_gpixel = textureLoad(previous_gbuffer, temporal_pixel_id, 0);
    let temporal_world_position = reconstruct_previous_world_position(temporal_pixel_id, temporal_depth);
    let temporal_world_normal = octahedral_decode(unpack_24bit_normal(temporal_gpixel.a));
    if pixel_dissimilar(depth, world_position, temporal_world_position, world_normal, temporal_world_normal) {
        return empty_reservoir();
    }

    let temporal_pixel_index = temporal_pixel_id.x + temporal_pixel_id.y * u32(view.viewport.z);
    var temporal_reservoir = gi_reservoirs_a[temporal_pixel_index];

    temporal_reservoir.confidence_weight = min(temporal_reservoir.confidence_weight, CONFIDENCE_WEIGHT_CAP);

    return temporal_reservoir;
}

fn reconstruct_world_position(pixel_id: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_id) + 0.5) / view.viewport.zw;
    let xy_ndc = (uv - vec2(0.5)) * vec2(2.0, -2.0);
    let world_pos = view.world_from_clip * vec4(xy_ndc, depth, 1.0);
    return world_pos.xyz / world_pos.w;
}

fn reconstruct_previous_world_position(pixel_id: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_id) + 0.5) / view.viewport.zw;
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

struct Reservoir {
    sample_point_world_position: vec3<f32>,
    weight_sum: f32,
    radiance: vec3<f32>,
    confidence_weight: f32,
    unbiased_contribution_weight: f32,
    padding1: f32,
    padding2: f32,
    padding3: f32,
}

fn empty_reservoir() -> Reservoir {
    return Reservoir(
        vec3(0.0),
        0.0,
        vec3(0.0),
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
    );
}
