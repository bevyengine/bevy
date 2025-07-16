// https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::pbr_deferred_types::unpack_24bit_normal
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_pbr::rgb9e5::rgb9e5_to_vec3_
#import bevy_pbr::utils::{rand_f, octahedral_decode}
#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_solari::sampling::{LightSample, generate_random_light_sample, calculate_light_contribution, trace_light_visibility, sample_disk}
#import bevy_solari::scene_bindings::{previous_frame_light_id_translations, LIGHT_NOT_PRESENT_THIS_FRAME}

@group(1) @binding(0) var view_output: texture_storage_2d<rgba16float, read_write>;
@group(1) @binding(1) var<storage, read_write> di_reservoirs_a: array<Reservoir>;
@group(1) @binding(2) var<storage, read_write> di_reservoirs_b: array<Reservoir>;
@group(1) @binding(5) var gbuffer: texture_2d<u32>;
@group(1) @binding(6) var depth_buffer: texture_depth_2d;
@group(1) @binding(7) var motion_vectors: texture_2d<f32>;
@group(1) @binding(8) var previous_gbuffer: texture_2d<u32>;
@group(1) @binding(9) var previous_depth_buffer: texture_depth_2d;
@group(1) @binding(10) var<uniform> view: View;
@group(1) @binding(11) var<uniform> previous_view: PreviousViewUniforms;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

const INITIAL_SAMPLES = 32u;
const SPATIAL_REUSE_RADIUS_PIXELS = 30.0;
const CONFIDENCE_WEIGHT_CAP = 20.0;

const NULL_RESERVOIR_SAMPLE = 0xFFFFFFFFu;

@compute @workgroup_size(8, 8, 1)
fn initial_and_temporal(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        di_reservoirs_b[pixel_index] = empty_reservoir();
        return;
    }
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let base_color = pow(unpack4x8unorm(gpixel.r).rgb, vec3(2.2));
    let diffuse_brdf = base_color / PI;

    let initial_reservoir = generate_initial_reservoir(world_position, world_normal, diffuse_brdf, &rng);
    let temporal_reservoir = load_temporal_reservoir(global_id.xy, depth, world_position, world_normal);
    let merge_result = merge_reservoirs(initial_reservoir, temporal_reservoir, world_position, world_normal, diffuse_brdf, &rng);

    di_reservoirs_b[pixel_index] = merge_result.merged_reservoir;
}

@compute @workgroup_size(8, 8, 1)
fn spatial_and_shade(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        di_reservoirs_a[pixel_index] = empty_reservoir();
        textureStore(view_output, global_id.xy, vec4(vec3(0.0), 1.0));
        return;
    }
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let base_color = pow(unpack4x8unorm(gpixel.r).rgb, vec3(2.2));
    let diffuse_brdf = base_color / PI;
    let emissive = rgb9e5_to_vec3_(gpixel.g);

    let input_reservoir = di_reservoirs_b[pixel_index];
    let spatial_reservoir = load_spatial_reservoir(global_id.xy, depth, world_position, world_normal, &rng);
    let merge_result = merge_reservoirs(input_reservoir, spatial_reservoir, world_position, world_normal, diffuse_brdf, &rng);
    let combined_reservoir = merge_result.merged_reservoir;

    di_reservoirs_a[pixel_index] = combined_reservoir;

    var pixel_color = merge_result.selected_sample_radiance * combined_reservoir.unbiased_contribution_weight * combined_reservoir.visibility;
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

        reservoir.visibility = trace_light_visibility(reservoir.sample, world_position);
    }

    reservoir.confidence_weight = 1.0;
    return reservoir;
}

fn load_temporal_reservoir(pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>) -> Reservoir {
    let motion_vector = textureLoad(motion_vectors, pixel_id, 0).xy;
    let temporal_pixel_id_float = round(vec2<f32>(pixel_id) - (motion_vector * view.viewport.zw));
    let temporal_pixel_id = vec2<u32>(temporal_pixel_id_float);

    // Check if the current pixel was off screen during the previous frame (current pixel is newly visible),
    // or if all temporal history should assumed to be invalid
    if any(temporal_pixel_id_float < vec2(0.0)) || any(temporal_pixel_id_float >= view.viewport.zw) || bool(constants.reset) {
        return empty_reservoir();
    }

    // Check if the pixel features have changed heavily between the current and previous frame
    let temporal_depth = textureLoad(previous_depth_buffer, temporal_pixel_id, 0);
    let temporal_gpixel = textureLoad(previous_gbuffer, temporal_pixel_id, 0);
    let temporal_world_position = reconstruct_previous_world_position(temporal_pixel_id, temporal_depth);
    let temporal_world_normal = octahedral_decode(unpack_24bit_normal(temporal_gpixel.a));
    if pixel_dissimilar(depth, world_position, temporal_world_position, world_normal, temporal_world_normal) {
        return empty_reservoir();
    }

    let temporal_pixel_index = temporal_pixel_id.x + temporal_pixel_id.y * u32(view.viewport.z);
    var temporal_reservoir = di_reservoirs_a[temporal_pixel_index];

    // Check if the light selected in the previous frame no longer exists in the current frame (e.g. entity despawned)
    temporal_reservoir.sample.light_id.x = previous_frame_light_id_translations[temporal_reservoir.sample.light_id.x];
    if temporal_reservoir.sample.light_id.x == LIGHT_NOT_PRESENT_THIS_FRAME {
        return empty_reservoir();
    }

    temporal_reservoir.confidence_weight = min(temporal_reservoir.confidence_weight, CONFIDENCE_WEIGHT_CAP);

    return temporal_reservoir;
}

fn load_spatial_reservoir(pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>, rng: ptr<function, u32>) -> Reservoir {
    let spatial_pixel_id = get_neighbor_pixel_id(pixel_id, rng);

    let spatial_depth = textureLoad(depth_buffer, spatial_pixel_id, 0);
    let spatial_gpixel = textureLoad(gbuffer, spatial_pixel_id, 0);
    let spatial_world_position = reconstruct_world_position(spatial_pixel_id, spatial_depth);
    let spatial_world_normal = octahedral_decode(unpack_24bit_normal(spatial_gpixel.a));
    if pixel_dissimilar(depth, world_position, spatial_world_position, world_normal, spatial_world_normal) {
        return empty_reservoir();
    }

    let spatial_pixel_index = spatial_pixel_id.x + spatial_pixel_id.y * u32(view.viewport.z);
    var spatial_reservoir = di_reservoirs_b[spatial_pixel_index];

    if reservoir_valid(spatial_reservoir) {
        spatial_reservoir.visibility = trace_light_visibility(spatial_reservoir.sample, world_position);
    }

    return spatial_reservoir;
}

fn get_neighbor_pixel_id(center_pixel_id: vec2<u32>, rng: ptr<function, u32>) -> vec2<u32> {
    var spatial_id = vec2<f32>(center_pixel_id) + sample_disk(SPATIAL_REUSE_RADIUS_PIXELS, rng);
    spatial_id = clamp(spatial_id, vec2(0.0), view.viewport.zw - 1.0);
    return vec2<u32>(spatial_id);
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

// Don't adjust the size of this struct without also adjusting DI_RESERVOIR_STRUCT_SIZE.
struct Reservoir {
    sample: LightSample,
    weight_sum: f32,
    confidence_weight: f32,
    unbiased_contribution_weight: f32,
    visibility: f32,
}

fn empty_reservoir() -> Reservoir {
    return Reservoir(
        LightSample(vec2(NULL_RESERVOIR_SAMPLE, 0u), vec2(0.0)),
        0.0,
        0.0,
        0.0,
        0.0
    );
}

fn reservoir_valid(reservoir: Reservoir) -> bool {
    return reservoir.sample.light_id.x != NULL_RESERVOIR_SAMPLE;
}

struct ReservoirMergeResult {
    merged_reservoir: Reservoir,
    selected_sample_radiance: vec3<f32>,
}

fn merge_reservoirs(
    canonical_reservoir: Reservoir,
    other_reservoir: Reservoir,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    diffuse_brdf: vec3<f32>,
    rng: ptr<function, u32>,
) -> ReservoirMergeResult {
    // TODO: Balance heuristic MIS weights
    let mis_weight_denominator = 1.0 / (canonical_reservoir.confidence_weight + other_reservoir.confidence_weight);

    let canonical_mis_weight = canonical_reservoir.confidence_weight * mis_weight_denominator;
    let canonical_target_function = reservoir_target_function(canonical_reservoir, world_position, world_normal, diffuse_brdf);
    let canonical_resampling_weight = canonical_mis_weight * (canonical_target_function.a * canonical_reservoir.unbiased_contribution_weight);

    let other_mis_weight = other_reservoir.confidence_weight * mis_weight_denominator;
    let other_target_function = reservoir_target_function(other_reservoir, world_position, world_normal, diffuse_brdf);
    let other_resampling_weight = other_mis_weight * (other_target_function.a * other_reservoir.unbiased_contribution_weight);

    var combined_reservoir = empty_reservoir();
    combined_reservoir.weight_sum = canonical_resampling_weight + other_resampling_weight;
    combined_reservoir.confidence_weight = canonical_reservoir.confidence_weight + other_reservoir.confidence_weight;

    // https://yusuketokuyoshi.com/papers/2024/Efficient_Visibility_Reuse_for_Real-time_ReSTIR_(Supplementary_Document).pdf
    combined_reservoir.visibility = max(0.0, (canonical_reservoir.visibility * canonical_resampling_weight
        + other_reservoir.visibility * other_resampling_weight) / combined_reservoir.weight_sum);

    if rand_f(rng) < other_resampling_weight / combined_reservoir.weight_sum {
        combined_reservoir.sample = other_reservoir.sample;

        let inverse_target_function = select(0.0, 1.0 / other_target_function.a, other_target_function.a > 0.0);
        combined_reservoir.unbiased_contribution_weight = combined_reservoir.weight_sum * inverse_target_function;

        return ReservoirMergeResult(combined_reservoir, other_target_function.rgb);
    } else {
        combined_reservoir.sample = canonical_reservoir.sample;

        let inverse_target_function = select(0.0, 1.0 / canonical_target_function.a, canonical_target_function.a > 0.0);
        combined_reservoir.unbiased_contribution_weight = combined_reservoir.weight_sum * inverse_target_function;

        return ReservoirMergeResult(combined_reservoir, canonical_target_function.rgb);
    }
}

fn reservoir_target_function(reservoir: Reservoir, world_position: vec3<f32>, world_normal: vec3<f32>, diffuse_brdf: vec3<f32>) -> vec4<f32> {
    if !reservoir_valid(reservoir) { return vec4(0.0); }
    let light_contribution = calculate_light_contribution(reservoir.sample, world_position, world_normal).radiance * reservoir.visibility;
    let target_function = luminance(light_contribution * diffuse_brdf);
    return vec4(light_contribution, target_function);
}
