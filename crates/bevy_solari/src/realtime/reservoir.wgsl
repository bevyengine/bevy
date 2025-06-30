// https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf

#define_import_path bevy_solari::reservoir

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::rand_f
#import bevy_solari::sampling::{LightSample, calculate_light_contribution}

const NULL_RESERVOIR_SAMPLE = 0xFFFFFFFFu;

// Don't adjust the size of this struct without also adjusting RESERVOIR_STRUCT_SIZE.
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
    let light_contribution = calculate_light_contribution(reservoir.sample, world_position, world_normal).radiance;
    let target_function = luminance(light_contribution * diffuse_brdf);
    return vec4(light_contribution, target_function);
}
