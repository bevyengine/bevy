import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance;
import bevy_pbr::render::utils::{rand_f, rand_range_u, sample_cosine_hemisphere};
import bevy_render::maths::PI;
import package::realtime::light_candidates::{generate_directional_light_candidate, generate_local_light_candidate, light_candidate_split, LightCandidate};
import package::realtime::atmosphere::calculate_light_contribution;
import package::scene::sampling::{area_to_solid_angle_pdf, emissive_light_area_pdf, local_light_count, power_heuristic, trace_visibility, sample_environment_map_light};
import package::scene::bindings::{trace_ray, resolve_ray_hit_full, RAY_T_MIN};
import package::realtime::world_cache_query::query_world_cache;
import package::realtime::bindings::{
    view,
    constants,
    world_cache,
};

enable wgpu_ray_query;

@if(WORLD_CACHE_FORCE_OPAQUE)
const WORLD_CACHE_RAY_FLAG = RAY_FLAG_FORCE_OPAQUE;
@if(!WORLD_CACHE_FORCE_OPAQUE)
const WORLD_CACHE_RAY_FLAG = RAY_FLAG_NONE;

@compute @workgroup_size(64, 1, 1)
fn update_world_cache(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(global_invocation_id) thread_id: vec3<u32>) {
    let active_cell_id = get_active_cell(thread_id.x);
    if active_cell_id == NO_ACTIVE_CELL { return; }

    let cell_index = world_cache.active_cell_indices[active_cell_id];
    let geometry_data = world_cache.geometry_data[cell_index];
    var rng = cell_index + constants.frame_rng;

    // Direct light
    var new_radiance = sample_random_light_ris(geometry_data.world_position, geometry_data.world_normal, workgroup_id.xy, &rng);

    // Indirect light
    new_radiance += trace_gi_ray(geometry_data.world_position, geometry_data.world_normal, cell_index, &rng);

    // Blend the new sample into the cell
    let old_radiance = world_cache.radiance[cell_index];
    let luminance_delta = world_cache.luminance_deltas[cell_index];

    // https://bsky.app/profile/gboisse.bsky.social/post/3m5blga3ftk2a
    let sample_count = min(old_radiance.a + 1.0, constants.world_cache_max_temporal_samples);
    let alpha = abs(luminance_delta) / max(luminance(old_radiance.rgb), 0.001);
    let max_sample_count = mix(constants.world_cache_max_temporal_samples, 1.0, pow(saturate(alpha), 1.0 / 8.0));
    let frames_between_updates = ceil(f32(world_cache.active_cells_count) / f32(constants.world_cache_max_cell_updates_per_frame));
    let blend_amount = max(1.0 / min(sample_count, max_sample_count), saturate(frames_between_updates / constants.world_cache_max_temporal_samples));

    let blended_radiance = mix(old_radiance.rgb, new_radiance, blend_amount);
    let blended_luminance_delta = mix(luminance_delta, luminance(blended_radiance) - luminance(old_radiance.rgb), 1.0 / 8.0);

    world_cache.radiance[cell_index] = vec4(blended_radiance, sample_count);
    world_cache.luminance_deltas[cell_index] = blended_luminance_delta;
}

const NO_ACTIVE_CELL = 0xFFFFFFFFu;

fn get_active_cell(thread_id: u32) -> u32 {
    let active_cells_count = world_cache.active_cells_count;
    if thread_id >= min(active_cells_count, constants.world_cache_max_cell_updates_per_frame) { return NO_ACTIVE_CELL; }

    return (world_cache.update_window_start + thread_id) % active_cells_count;
}

fn sample_random_light_ris(world_position: vec3<f32>, world_normal: vec3<f32>, workgroup_id: vec2<u32>, rng: ptr<function, u32>) -> vec3<f32> {
    let split = light_candidate_split(constants.world_cache_local_light_sample_count);
    var workgroup_rng = (workgroup_id.x * 5782582u) + workgroup_id.y;
    let light_tile_start = rand_range_u(128u, &workgroup_rng) * 1024u;

    var ris: WorldCacheLightRisState;
    for (var i = 0u; i < split.directional; i++) {
        add_world_cache_light_ris_candidate(&ris, generate_directional_light_candidate(split, rng), world_position, world_normal, rng);
    }
    for (var i = 0u; i < split.local; i++) {
        add_world_cache_light_ris_candidate(&ris, generate_local_light_candidate(split, light_tile_start, rng), world_position, world_normal, rng);
    }

    var unbiased_contribution_weight = 0.0;
    if any(ris.selected_sample_radiance != vec3(0.0)) {
        let inverse_target_function = select(0.0, 1.0 / ris.selected_sample_target_function, ris.selected_sample_target_function > 0.0);
        unbiased_contribution_weight = ris.weight_sum * inverse_target_function;

        unbiased_contribution_weight *= trace_visibility(world_position + (world_normal * RAY_T_MIN), ris.selected_sample_world_position, WORLD_CACHE_RAY_FLAG);

        if ris.selected_brdf_rays_can_hit && ris.selected_inverse_solid_angle_pdf > 0.0
            && distance(ris.selected_sample_world_position.xyz, world_position) < constants.world_cache_max_gi_ray_distance {
            let p_nee_strategy = 1.0 / ris.selected_inverse_solid_angle_pdf;
            let p_brdf_at_nee = saturate(dot(world_normal, ris.selected_wi)) / PI;
            let nee_mis_weight = power_heuristic(p_nee_strategy, p_brdf_at_nee);
            unbiased_contribution_weight *= nee_mis_weight;
        }
    }

    return ris.selected_sample_radiance * unbiased_contribution_weight;
}

struct WorldCacheLightRisState {
    weight_sum: f32,
    selected_sample_radiance: vec3<f32>,
    selected_sample_target_function: f32,
    selected_sample_world_position: vec4<f32>,
    selected_wi: vec3<f32>,
    selected_inverse_solid_angle_pdf: f32,
    selected_brdf_rays_can_hit: bool,
}

fn add_world_cache_light_ris_candidate(ris: ptr<function, WorldCacheLightRisState>, candidate: LightCandidate, world_position: vec3<f32>, world_normal: vec3<f32>, rng: ptr<function, u32>) {
    let resolved_light_sample = candidate.resolved_light_sample;
    let light_contribution = calculate_light_contribution(resolved_light_sample, world_position, world_normal);

    let contribution = light_contribution.radiance * saturate(dot(light_contribution.wi, world_normal));
    let target_function = luminance(contribution);
    let resampling_weight = target_function * light_contribution.inverse_pdf;

    (*ris).weight_sum += resampling_weight;

    if rand_f(rng) * (*ris).weight_sum < resampling_weight {
        (*ris).selected_sample_radiance = contribution;
        (*ris).selected_sample_target_function = target_function;
        (*ris).selected_sample_world_position = resolved_light_sample.world_position;
        (*ris).selected_wi = light_contribution.wi;
        (*ris).selected_inverse_solid_angle_pdf = light_contribution.inverse_solid_angle_pdf;
        (*ris).selected_brdf_rays_can_hit = light_contribution.brdf_rays_can_hit;
    }
}

fn trace_gi_ray(world_position: vec3<f32>, world_normal: vec3<f32>, cell_index: u32, rng: ptr<function, u32>) -> vec3<f32> {
    let ray_direction = sample_cosine_hemisphere(world_normal, rng);
    let ray = trace_ray(world_position + (world_normal * RAY_T_MIN), ray_direction, RAY_T_MIN, constants.world_cache_max_gi_ray_distance, WORLD_CACHE_RAY_FLAG);
    if ray.kind == RAY_QUERY_INTERSECTION_NONE {
        // The ray is cut short at world_cache_max_gi_ray_distance to save on traversal costs.
        // A miss here may actually have been a surface further away if we kept tracing, and not actually have been a miss.
        // We could be leaking sky light due to this if world_cache_max_gi_ray_distance is too small.
        return PI * sample_environment_map_light(ray_direction);
    }

    let ray_hit = resolve_ray_hit_full(ray, -ray_direction);
    var radiance = vec3(0.0);

    let NdotV_hit = dot(ray_hit.world_normal, -ray_direction);
    if any(ray_hit.material.emissive > vec3(0.0)) && NdotV_hit > 0.0 {
        let split = light_candidate_split(constants.world_cache_local_light_sample_count);
        let p_light = area_to_solid_angle_pdf(emissive_light_area_pdf(ray_hit, local_light_count()), ray.t, NdotV_hit);
        let p_brdf = saturate(dot(world_normal, ray_direction)) / PI;
        let emissive_mis_weight = power_heuristic(p_brdf, p_light * f32(split.local));
        radiance += PI * ray_hit.material.emissive * emissive_mis_weight;
    }

    if ray.front_face || ray_hit.double_sided {
        let cell_life = atomicLoad(&world_cache.life[cell_index]);
        let cached_radiance = query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal, view.world_position, cell_life, rng);
        radiance += ray_hit.material.base_color * cached_radiance;
    }

    return radiance;
}
