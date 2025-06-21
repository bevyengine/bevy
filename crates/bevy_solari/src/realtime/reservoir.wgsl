// https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf

#define_import_path bevy_solari::reservoir

#import bevy_solari::sampling::LightSample

const NULL_RESERVOIR_SAMPLE = 0xFFFFFFFFu;

// Don't adjust the size of this struct without also adjusting RESERVOIR_STRUCT_SIZE.
struct Reservoir {
    sample: LightSample,
    weight_sum: f32,
    confidence_weight: f32,
    unbiased_contribution_weight: f32,
    _padding: f32,
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
