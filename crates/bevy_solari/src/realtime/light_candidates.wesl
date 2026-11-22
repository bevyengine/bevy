import bevy_pbr::render::utils::rand_range_u;
import package::realtime::bindings::{light_tile_resolved_samples, light_tile_samples, view};
import package::scene::bindings::directional_light_count;
import package::realtime::presample_light_tiles::unpack_resolved_light_sample;
import package::scene::sampling::{generate_directional_light_sample, local_light_count, LightSample, NULL_LIGHT_ID, ResolvedLightSample};

enable wgpu_ray_query;

/// Directional light candidates per RIS loop, on top of the local light candidates
const DIRECTIONAL_CANDIDATES: u32 = 1u;

struct LightCandidateSplit {
    directional: u32,
    local: u32,
}

fn light_candidate_split(candidate_count: u32) -> LightCandidateSplit {
    let directional = select(DIRECTIONAL_CANDIDATES, 0u, directional_light_count == 0u);
    let local = select(candidate_count, 0u, local_light_count() == 0u);
    return LightCandidateSplit(directional, local);
}

fn local_light_selection_pdf(split: LightCandidateSplit) -> f32 {
    return f32(split.local) / f32(max(local_light_count(), 1u));
}

/// Light tile slot of directional light candidates, which don't come from the light tiles
const NO_LIGHT_TILE_SLOT: u32 = 0xFFFFFFFFu;

struct LightCandidate {
    /// Only set for directional lights, local lights get it from their light tile slot via `candidate_light_sample`
    light_sample: LightSample,
    light_tile_slot: u32,
    resolved_light_sample: ResolvedLightSample,
}

/// A directional light candidate, picked uniformly (directional lights come first in light_sources).
/// Only call this when `split.directional > 0`.
fn generate_directional_light_candidate(split: LightCandidateSplit, rng: ptr<function, u32>) -> LightCandidate {
    let light_id = rand_range_u(directional_light_count, rng);
    var sample = generate_directional_light_sample(light_id, rng);
    sample.resolved_light_sample.inverse_pdf *= f32(directional_light_count) / f32(split.directional);
    return LightCandidate(sample.light_sample, NO_LIGHT_TILE_SLOT, sample.resolved_light_sample);
}

/// A local light candidate, from the light tiles. Only call this when `split.local > 0`.
fn generate_local_light_candidate(split: LightCandidateSplit, light_tile_start: u32, rng: ptr<function, u32>) -> LightCandidate {
    let tile_sample = light_tile_start + rand_range_u(1024u, rng);
    var resolved_light_sample = unpack_resolved_light_sample(light_tile_resolved_samples[tile_sample], view.exposure);
    resolved_light_sample.inverse_pdf /= local_light_selection_pdf(split);
    return LightCandidate(LightSample(NULL_LIGHT_ID, 0u), tile_sample, resolved_light_sample);
}

fn candidate_light_sample(light_sample: LightSample, light_tile_slot: u32) -> LightSample {
    if light_tile_slot == NO_LIGHT_TILE_SLOT { return light_sample; }
    return light_tile_samples[light_tile_slot];
}
