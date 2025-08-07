// https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf
// https://d1qx31qr3h6wln.cloudfront.net/publications/ReSTIR%20GI.pdf

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::pbr_deferred_types::unpack_24bit_normal
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_pbr::rgb9e5::rgb9e5_to_vec3_
#import bevy_pbr::utils::{rand_f, rand_range_u, octahedral_decode, sample_disk}
#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_solari::presample_light_tiles::{ResolvedLightSamplePacked, unpack_resolved_light_sample}
#import bevy_solari::sampling::{LightSample, calculate_resolved_light_contribution, resolve_and_calculate_light_contribution, resolve_light_sample, trace_light_visibility}
#import bevy_solari::scene_bindings::{light_sources, previous_frame_light_id_translations, LIGHT_NOT_PRESENT_THIS_FRAME}

@group(1) @binding(0) var view_output: texture_storage_2d<rgba16float, read_write>;
@group(1) @binding(1) var<storage, read_write> light_tile_samples: array<LightSample>;
@group(1) @binding(2) var<storage, read_write> light_tile_resolved_samples: array<ResolvedLightSamplePacked>;
@group(1) @binding(3) var di_reservoirs_a: texture_storage_2d<rgba32uint, read_write>;
@group(1) @binding(4) var di_reservoirs_b: texture_storage_2d<rgba32uint, read_write>;
@group(1) @binding(7) var gbuffer: texture_2d<u32>;
@group(1) @binding(8) var depth_buffer: texture_depth_2d;
@group(1) @binding(9) var motion_vectors: texture_2d<f32>;
@group(1) @binding(10) var previous_gbuffer: texture_2d<u32>;
@group(1) @binding(11) var previous_depth_buffer: texture_depth_2d;
@group(1) @binding(12) var<uniform> view: View;
@group(1) @binding(13) var<uniform> previous_view: PreviousViewUniforms;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

const INITIAL_SAMPLES = 32u;
const SPATIAL_REUSE_RADIUS_PIXELS = 30.0;
const CONFIDENCE_WEIGHT_CAP = 20.0;

const NULL_RESERVOIR_SAMPLE = 0xFFFFFFFFu;

@compute @workgroup_size(8, 8, 1)
fn initial_and_temporal(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.main_pass_viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.main_pass_viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        store_reservoir_b(global_id.xy, empty_reservoir());
        return;
    }
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let base_color = pow(unpack4x8unorm(gpixel.r).rgb, vec3(2.2));
    let diffuse_brdf = base_color / PI;

    let initial_reservoir = generate_initial_reservoir(world_position, world_normal, diffuse_brdf, workgroup_id.xy, &rng);
    let temporal_reservoir = load_temporal_reservoir(global_id.xy, depth, world_position, world_normal);
    let merge_result = merge_reservoirs(initial_reservoir, temporal_reservoir, world_position, world_normal, diffuse_brdf, &rng);

    store_reservoir_b(global_id.xy, merge_result.merged_reservoir);
}

@compute @workgroup_size(8, 8, 1)
fn spatial_and_shade(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.main_pass_viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.main_pass_viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        store_reservoir_a(global_id.xy, empty_reservoir());
        textureStore(view_output, global_id.xy, vec4(vec3(0.0), 1.0));
        return;
    }
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let base_color = pow(unpack4x8unorm(gpixel.r).rgb, vec3(2.2));
    let diffuse_brdf = base_color / PI;
    let emissive = rgb9e5_to_vec3_(gpixel.g);

    let input_reservoir = load_reservoir_b(global_id.xy);
    let spatial_reservoir = load_spatial_reservoir(global_id.xy, depth, world_position, world_normal, &rng);
    let merge_result = merge_reservoirs(input_reservoir, spatial_reservoir, world_position, world_normal, diffuse_brdf, &rng);
    let combined_reservoir = merge_result.merged_reservoir;

    store_reservoir_a(global_id.xy, combined_reservoir);

    var pixel_color = merge_result.selected_sample_radiance * combined_reservoir.unbiased_contribution_weight;
    pixel_color *= view.exposure;
    pixel_color *= diffuse_brdf;
    pixel_color += emissive;
    textureStore(view_output, global_id.xy, vec4(pixel_color, 1.0));
}

fn generate_initial_reservoir(world_position: vec3<f32>, world_normal: vec3<f32>, diffuse_brdf: vec3<f32>, workgroup_id: vec2<u32>, rng: ptr<function, u32>) -> Reservoir {
    var workgroup_rng = (workgroup_id.x * 5782582u) + workgroup_id.y;
    let light_tile_start = rand_range_u(128u, &workgroup_rng) * 1024u;

    var reservoir = empty_reservoir();
    var weight_sum = 0.0;
    let mis_weight = 1.0 / f32(INITIAL_SAMPLES);

    var reservoir_target_function = 0.0;
    var light_sample_world_position = vec4(0.0);
    var selected_tile_sample = 0u;
    for (var i = 0u; i < INITIAL_SAMPLES; i++) {
        let tile_sample = light_tile_start + rand_range_u(1024u, rng);
        let resolved_light_sample = unpack_resolved_light_sample(light_tile_resolved_samples[tile_sample], view.exposure);
        let light_contribution = calculate_resolved_light_contribution(resolved_light_sample, world_position, world_normal);

        let target_function = luminance(light_contribution.radiance * diffuse_brdf);
        let resampling_weight = mis_weight * (target_function * light_contribution.inverse_pdf);

        weight_sum += resampling_weight;

        if rand_f(rng) < resampling_weight / weight_sum {
            reservoir_target_function = target_function;
            light_sample_world_position = resolved_light_sample.world_position;
            selected_tile_sample = tile_sample;
        }
    }

    if reservoir_target_function != 0.0 {
        reservoir.sample = light_tile_samples[selected_tile_sample];
    }

    if reservoir_valid(reservoir) {
        let inverse_target_function = select(0.0, 1.0 / reservoir_target_function, reservoir_target_function > 0.0);
        reservoir.unbiased_contribution_weight = weight_sum * inverse_target_function;

        reservoir.unbiased_contribution_weight *= trace_light_visibility(world_position, light_sample_world_position);
    }

    reservoir.confidence_weight = 1.0;
    return reservoir;
}

fn load_temporal_reservoir(pixel_id: vec2<u32>, depth: f32, world_position: vec3<f32>, world_normal: vec3<f32>) -> Reservoir {
    let motion_vector = textureLoad(motion_vectors, pixel_id, 0).xy;
    let temporal_pixel_id_float = round(vec2<f32>(pixel_id) - (motion_vector * view.main_pass_viewport.zw));
    let temporal_pixel_id = vec2<u32>(temporal_pixel_id_float);

    // Check if the current pixel was off screen during the previous frame (current pixel is newly visible),
    // or if all temporal history should assumed to be invalid
    if any(temporal_pixel_id_float < vec2(0.0)) || any(temporal_pixel_id_float >= view.main_pass_viewport.zw) || bool(constants.reset) {
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

    var temporal_reservoir = load_reservoir_a(temporal_pixel_id);

    // Check if the light selected in the previous frame no longer exists in the current frame (e.g. entity despawned)
    let previous_light_id = temporal_reservoir.sample.light_id >> 16u;
    let triangle_id = temporal_reservoir.sample.light_id & 0xFFFFu;
    let light_id = previous_frame_light_id_translations[previous_light_id];
    if light_id == LIGHT_NOT_PRESENT_THIS_FRAME {
        return empty_reservoir();
    }
    temporal_reservoir.sample.light_id = (light_id << 16u) | triangle_id;

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

    var spatial_reservoir = load_reservoir_b(spatial_pixel_id);

    if reservoir_valid(spatial_reservoir) {
        let resolved_light_sample = resolve_light_sample(spatial_reservoir.sample, light_sources[spatial_reservoir.sample.light_id >> 16u]);
        spatial_reservoir.unbiased_contribution_weight *= trace_light_visibility(world_position, resolved_light_sample.world_position);
    }

    return spatial_reservoir;
}

fn get_neighbor_pixel_id(center_pixel_id: vec2<u32>, rng: ptr<function, u32>) -> vec2<u32> {
    var spatial_id = vec2<f32>(center_pixel_id) + sample_disk(SPATIAL_REUSE_RADIUS_PIXELS, rng);
    spatial_id = clamp(spatial_id, vec2(0.0), view.main_pass_viewport.zw - 1.0);
    return vec2<u32>(spatial_id);
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
}

fn merge_reservoirs(
    canonical_reservoir: Reservoir,
    other_reservoir: Reservoir,
    world_position: vec3<f32>,
    world_normal: vec3<f32>,
    diffuse_brdf: vec3<f32>,
    rng: ptr<function, u32>,
) -> ReservoirMergeResult {
    let mis_weight_denominator = 1.0 / (canonical_reservoir.confidence_weight + other_reservoir.confidence_weight);

    let canonical_mis_weight = canonical_reservoir.confidence_weight * mis_weight_denominator;
    let canonical_target_function = reservoir_target_function(canonical_reservoir, world_position, world_normal, diffuse_brdf);
    let canonical_resampling_weight = canonical_mis_weight * (canonical_target_function.a * canonical_reservoir.unbiased_contribution_weight);

    let other_mis_weight = other_reservoir.confidence_weight * mis_weight_denominator;
    let other_target_function = reservoir_target_function(other_reservoir, world_position, world_normal, diffuse_brdf);
    let other_resampling_weight = other_mis_weight * (other_target_function.a * other_reservoir.unbiased_contribution_weight);

    let weight_sum = canonical_resampling_weight + other_resampling_weight;

    var combined_reservoir = empty_reservoir();
    combined_reservoir.confidence_weight = canonical_reservoir.confidence_weight + other_reservoir.confidence_weight;

    if rand_f(rng) < other_resampling_weight / weight_sum {
        combined_reservoir.sample = other_reservoir.sample;

        let inverse_target_function = select(0.0, 1.0 / other_target_function.a, other_target_function.a > 0.0);
        combined_reservoir.unbiased_contribution_weight = weight_sum * inverse_target_function;

        return ReservoirMergeResult(combined_reservoir, other_target_function.rgb);
    } else {
        combined_reservoir.sample = canonical_reservoir.sample;

        let inverse_target_function = select(0.0, 1.0 / canonical_target_function.a, canonical_target_function.a > 0.0);
        combined_reservoir.unbiased_contribution_weight = weight_sum * inverse_target_function;

        return ReservoirMergeResult(combined_reservoir, canonical_target_function.rgb);
    }
}

// TODO: Have input take ResolvedLightSample instead of reservoir.light_sample
fn reservoir_target_function(reservoir: Reservoir, world_position: vec3<f32>, world_normal: vec3<f32>, diffuse_brdf: vec3<f32>) -> vec4<f32> {
    if !reservoir_valid(reservoir) { return vec4(0.0); }
    let light_contribution = resolve_and_calculate_light_contribution(reservoir.sample, world_position, world_normal).radiance;
    let target_function = luminance(light_contribution * diffuse_brdf);
    return vec4(light_contribution, target_function);
}
