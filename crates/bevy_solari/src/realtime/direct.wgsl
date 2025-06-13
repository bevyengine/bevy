#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::pbr_deferred_types::unpack_24bit_normal
#import bevy_pbr::rgb9e5::rgb9e5_to_vec3_
#import bevy_pbr::utils::{rand_f, octahedral_decode}
#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_solari::reservoir::{Reservoir, empty_reservoir, reservoir_valid}
#import bevy_solari::sampling::{generate_random_light_sample, calculate_light_contribution, trace_light_visibility, sample_disk}

@group(1) @binding(0) var view_output: texture_storage_2d<rgba16float, write>;
@group(1) @binding(1) var<storage, read> previous_reservoirs: array<Reservoir>;
@group(1) @binding(2) var<storage, read_write> reservoirs: array<Reservoir>;
@group(1) @binding(3) var gbuffer: texture_2d<u32>;
@group(1) @binding(4) var depth_buffer: texture_depth_2d;
@group(1) @binding(5) var motion_vectors: texture_2d<f32>;
@group(1) @binding(6) var<uniform> view: View;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

const INITIAL_SAMPLES = 32u;
const SPATIAL_REUSE_RADIUS_PIXELS = 30.0;
const CONFIDENCE_WEIGHT_CAP = 20.0;

@compute @workgroup_size(8, 8, 1)
fn initial_samples(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        reservoirs[pixel_index] = empty_reservoir();
        return;
    }
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));

    var reservoir = empty_reservoir();
    var reservoir_target_function = 0.0;
    for (var i = 0u; i < INITIAL_SAMPLES; i++) {
        let light_sample = generate_random_light_sample(&rng);

        let mis_weight = 1.0 / f32(INITIAL_SAMPLES);
        let light_contribution = calculate_light_contribution(light_sample, world_position, world_normal);
        let target_function = luminance(light_contribution.radiance);
        let resampling_weight = mis_weight * (target_function * light_contribution.inverse_pdf);

        reservoir.weight_sum += resampling_weight;

        if rand_f(&rng) < resampling_weight / reservoir.weight_sum {
            reservoir.sample = light_sample;
            reservoir_target_function = target_function;
        }
    }

    if reservoir_valid(reservoir) {
        let inverse_target_function = select(0.0, 1.0 / reservoir_target_function, reservoir_target_function > 0.0);
        reservoir.unbiased_contribution_weight = reservoir.weight_sum * inverse_target_function;
        reservoir.unbiased_contribution_weight *= trace_light_visibility(reservoir.sample, world_position);
    }

    reservoir.confidence_weight = 1.0;
    reservoirs[pixel_index] = reservoir;
}

@compute @workgroup_size(8, 8, 1)
fn spatial_reuse(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        textureStore(view_output, global_id.xy, vec4(vec3(0.0), 1.0));
        return;
    }
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let base_color = pow(unpack4x8unorm(gpixel.r).rgb, vec3(2.2));
    let diffuse_brdf = base_color / PI;
    let emissive = rgb9e5_to_vec3_(gpixel.g);

    let neighbor_pixel_id = get_neighbor_pixel_id(global_id.xy, &rng);
    let neighbor_pixel_index = neighbor_pixel_id.x + neighbor_pixel_id.y * u32(view.viewport.z);
    let neighbor_depth = textureLoad(depth_buffer, neighbor_pixel_id, 0);
    let neighbor_gpixel = textureLoad(gbuffer, neighbor_pixel_id, 0);
    let neighbor_world_position = reconstruct_world_position(neighbor_pixel_id, neighbor_depth);
    let neighbor_world_normal = octahedral_decode(unpack_24bit_normal(neighbor_gpixel.a));
    let neighbor_valid = !is_neighbor_invalid(depth, neighbor_depth, world_normal, neighbor_world_normal);
    let neighbor_reservoir = reservoirs[neighbor_pixel_index];
    let neighbor_reservoir_confidence = select(0.0, neighbor_reservoir.confidence_weight * f32(INITIAL_SAMPLES), neighbor_valid);

    let input_reservoir = reservoirs[pixel_index];
    let input_reservoir_confidence = input_reservoir.confidence_weight * f32(INITIAL_SAMPLES);
    let input_reservoir_radiance = select(
        vec3(0.0),
        calculate_light_contribution(input_reservoir.sample, world_position, world_normal).radiance,
        reservoir_valid(input_reservoir),
    );

    var combined_reservoir = empty_reservoir();

    let input_target_function = luminance(input_reservoir_radiance);
#ifdef BIASED
    let mis_weight = input_reservoir_confidence / (input_reservoir_confidence + neighbor_reservoir_confidence);
#else
    let neighbor_target_function = select(0.0, reservoir_target_function(input_reservoir, neighbor_world_position, neighbor_world_normal), neighbor_valid);
    let mis_weight = max(0.0, (input_reservoir_confidence * input_target_function) / ((input_reservoir_confidence * input_target_function) + (neighbor_reservoir_confidence * neighbor_target_function)));
#endif
    let resampling_weight = mis_weight * (input_target_function * input_reservoir.unbiased_contribution_weight);

    combined_reservoir.weight_sum += resampling_weight;
    combined_reservoir.confidence_weight += input_reservoir.confidence_weight;

    if rand_f(&rng) < resampling_weight / combined_reservoir.weight_sum {
        combined_reservoir.sample = input_reservoir.sample;
    }

    if neighbor_valid {
        let input_target_function = reservoir_target_function(neighbor_reservoir, world_position, world_normal);
#ifdef BIASED
        let mis_weight = neighbor_reservoir_confidence / (input_reservoir_confidence + neighbor_reservoir_confidence);
#else
        let neighbor_target_function = reservoir_target_function(neighbor_reservoir, neighbor_world_position, neighbor_world_normal);
        let mis_weight = max(0.0, (neighbor_reservoir_confidence * neighbor_target_function) / ((input_reservoir_confidence * input_target_function) + (neighbor_reservoir_confidence * neighbor_target_function)));
#endif
        let resampling_weight = mis_weight * (input_target_function * neighbor_reservoir.unbiased_contribution_weight);

        combined_reservoir.weight_sum += resampling_weight;
        combined_reservoir.confidence_weight += neighbor_reservoir.confidence_weight;

        if rand_f(&rng) < resampling_weight / combined_reservoir.weight_sum {
            combined_reservoir.sample = neighbor_reservoir.sample;
        }
    }

    var combined_reservoir_radiance = vec3(0.0);
    if reservoir_valid(combined_reservoir) {
        combined_reservoir_radiance = calculate_light_contribution(combined_reservoir.sample, world_position, world_normal).radiance;
        let target_function = luminance(combined_reservoir_radiance);
        let inverse_target_function = select(0.0, 1.0 / target_function, target_function > 0.0);
        combined_reservoir.unbiased_contribution_weight = combined_reservoir.weight_sum * inverse_target_function;
        combined_reservoir.unbiased_contribution_weight *= trace_light_visibility(combined_reservoir.sample, world_position);
    }

    combined_reservoir.confidence_weight = min(combined_reservoir.confidence_weight, CONFIDENCE_WEIGHT_CAP);
    reservoirs[pixel_index] = combined_reservoir;

    var pixel_color = input_reservoir_radiance * input_reservoir.unbiased_contribution_weight;
    pixel_color += combined_reservoir_radiance * combined_reservoir.unbiased_contribution_weight;
    pixel_color *= 0.5;
    pixel_color *= view.exposure;
    pixel_color *= diffuse_brdf;
    pixel_color += emissive;
    textureStore(view_output, global_id.xy, vec4(pixel_color, 1.0));
}

fn reservoir_target_function(reservoir: Reservoir, world_position: vec3<f32>, world_normal: vec3<f32>) -> f32 {
    if !reservoir_valid(reservoir) { return 0.0; }
    return luminance(calculate_light_contribution(reservoir.sample, world_position, world_normal).radiance);
}

fn get_neighbor_pixel_id(center_pixel_id: vec2<u32>, rng: ptr<function, u32>) -> vec2<u32> {
    var neighbor_id = vec2<i32>(center_pixel_id) + vec2<i32>(sample_disk(SPATIAL_REUSE_RADIUS_PIXELS, rng));
    neighbor_id = clamp(neighbor_id, vec2(0i), vec2<i32>(view.viewport.zw) - 1i);
    return vec2<u32>(neighbor_id);
}

fn reconstruct_world_position(pixel_id: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_id) + 0.5) / view.viewport.zw;
    let xy_ndc = (uv - vec2(0.5)) * vec2(2.0, -2.0);
    let world_pos = view.world_from_clip * vec4(xy_ndc, depth, 1.0);
    return world_pos.xyz / world_pos.w;
}

// TODO: Plane distance instead of depth
// https://developer.download.nvidia.com/video/gputechconf/gtc/2020/presentations/s22699-fast-denoising-with-self-stabilizing-recurrent-blurs.pdf#page=45
fn is_neighbor_invalid(depth: f32, neighbor_depth: f32, normal: vec3<f32>, neighbor_normal: vec3<f32>) -> bool {
    let linear_depth = -depth_ndc_to_view_z(depth);
    let linear_neighbor_depth = -depth_ndc_to_view_z(neighbor_depth);

    // Reject if depth difference more than 10% or angle between normals more than 25 degrees
    return linear_neighbor_depth > 1.1 * linear_depth || linear_neighbor_depth < 0.9 * linear_depth ||
        dot(normal, neighbor_normal) < 0.906;
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
