#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::pbr_deferred_types::unpack_24bit_normal
#import bevy_pbr::rgb9e5::rgb9e5_to_vec3_
#import bevy_pbr::utils::{rand_f, octahedral_decode}
#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_solari::reservoir::{Reservoir, empty_reservoir, reservoir_valid}
#import bevy_solari::sampling::{generate_random_light_sample, calculate_light_contribution, trace_light_visibility}

@group(1) @binding(0) var view_output: texture_storage_2d<rgba16float, write>;
@group(1) @binding(1) var<storage, read_write> reservoirs_a: array<Reservoir>;
@group(1) @binding(2) var<storage, read_write> reservoirs_b: array<Reservoir>;
@group(1) @binding(3) var gbuffer: texture_2d<u32>;
@group(1) @binding(4) var depth_buffer: texture_depth_2d;
@group(1) @binding(5) var motion_vectors: texture_2d<f32>;
@group(1) @binding(6) var<uniform> view: View;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

const INITIAL_SAMPLES = 32u;
const SPATIAL_REUSE_RADIUS_PIXELS = 30.0;
const CONFIDENCE_WEIGHT_CAP = 20.0 * f32(INITIAL_SAMPLES);

@compute @workgroup_size(8, 8, 1)
fn initial_and_temporal(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        reservoirs_b[pixel_index] = empty_reservoir();
        return;
    }
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let base_color = pow(unpack4x8unorm(gpixel.r).rgb, vec3(2.2));
    let diffuse_brdf = base_color / PI;

    let initial_reservoir = generate_initial_reservoir(world_position, world_normal, diffuse_brdf, &rng);

    reservoirs_b[pixel_index] = initial_reservoir;
}

@compute @workgroup_size(8, 8, 1)
fn spatial_and_shade(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        reservoirs_a[pixel_index] = empty_reservoir();
        textureStore(view_output, global_id.xy, vec4(vec3(0.0), 1.0));
        return;
    }
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let base_color = pow(unpack4x8unorm(gpixel.r).rgb, vec3(2.2));
    let diffuse_brdf = base_color / PI;
    let emissive = rgb9e5_to_vec3_(gpixel.g);

    let input_reservoir = reservoirs_b[pixel_index];

    var radiance = vec3(0.0);
    if reservoir_valid(input_reservoir) {
        radiance = calculate_light_contribution(input_reservoir.sample, world_position, world_normal).radiance;
    }

    reservoirs_a[pixel_index] = input_reservoir;

    var pixel_color = radiance * input_reservoir.unbiased_contribution_weight;
    pixel_color *= view.exposure;
    pixel_color *= diffuse_brdf;
    pixel_color += emissive;
    textureStore(view_output, global_id.xy, vec4(pixel_color, 1.0));
}

fn generate_initial_reservoir(world_position: vec3<f32>, world_normal: vec3<f32>, diffuse_brdf: vec3<f32>, rng: ptr<function, u32>) -> Reservoir{
    var reservoir = empty_reservoir();
    var reservoir_target_function = 0.0;
    for (var i = 0u; i < INITIAL_SAMPLES; i++) {
        let light_sample = generate_random_light_sample(rng);

        let mis_weight = 1.0 / f32(INITIAL_SAMPLES);
        let light_contribution = calculate_light_contribution(light_sample, world_position, world_normal);
        let target_function = luminance(light_contribution.radiance * diffuse_brdf);
        let resampling_weight = mis_weight * (target_function * light_contribution.inverse_pdf);

        reservoir.weight_sum += resampling_weight;

        if rand_f(rng) < resampling_weight / reservoir.weight_sum {
            reservoir.sample = light_sample;
            reservoir_target_function = target_function;
        }
    }

    if reservoir_valid(reservoir) {
        let inverse_target_function = select(0.0, 1.0 / reservoir_target_function, reservoir_target_function > 0.0);
        reservoir.unbiased_contribution_weight = reservoir.weight_sum * inverse_target_function;
        reservoir.unbiased_contribution_weight *= trace_light_visibility(reservoir.sample, world_position);
    }

    reservoir.confidence_weight = f32(INITIAL_SAMPLES);
    return reservoir;
}

fn reconstruct_world_position(pixel_id: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_id) + 0.5) / view.viewport.zw;
    let xy_ndc = (uv - vec2(0.5)) * vec2(2.0, -2.0);
    let world_pos = view.world_from_clip * vec4(xy_ndc, depth, 1.0);
    return world_pos.xyz / world_pos.w;
}
