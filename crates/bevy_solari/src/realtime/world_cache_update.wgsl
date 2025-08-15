#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::{rand_f, rand_range_u, sample_cosine_hemisphere}
#import bevy_render::view::View
#import bevy_solari::presample_light_tiles::{ResolvedLightSamplePacked, unpack_resolved_light_sample}
#import bevy_solari::sampling::{calculate_resolved_light_contribution, trace_light_visibility}
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, RAY_T_MIN, RAY_T_MAX}
#import bevy_solari::world_cache::{
    WORLD_CACHE_MAX_TEMPORAL_SAMPLES,
    query_world_cache,
    world_cache_active_cells_count,
    world_cache_active_cell_indices,
    world_cache_geometry_data,
    world_cache_radiance,
    world_cache_active_cells_new_radiance,
}

@group(1) @binding(2) var<storage, read_write> light_tile_resolved_samples: array<ResolvedLightSamplePacked>;
@group(1) @binding(12) var<uniform> view: View;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

const DIRECT_LIGHT_SAMPLE_COUNT: u32 = 32u;

@compute @workgroup_size(1024, 1, 1)
fn sample_radiance(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    if active_cell_id.x < world_cache_active_cells_count {
        let cell_index = world_cache_active_cell_indices[active_cell_id.x];
        let geometry_data = world_cache_geometry_data[cell_index];
        var rng = cell_index + constants.frame_index;

        // TODO: Initialize newly active cells with data from an adjacent LOD

        var new_radiance = sample_random_light_ris(geometry_data.world_position, geometry_data.world_normal, workgroup_id.xy, &rng);

#ifndef NO_MULTIBOUNCE
        let ray_direction = sample_cosine_hemisphere(geometry_data.world_normal, &rng);
        let ray_hit = trace_ray(geometry_data.world_position, ray_direction, RAY_T_MIN, RAY_T_MAX, RAY_FLAG_NONE);
        if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
            let ray_hit = resolve_ray_hit_full(ray_hit);
            new_radiance += ray_hit.material.base_color * query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal, view.world_position);
        }
#endif

        world_cache_active_cells_new_radiance[active_cell_id.x] = new_radiance;
    }
}

@compute @workgroup_size(1024, 1, 1)
fn blend_new_samples(@builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    if active_cell_id.x < world_cache_active_cells_count {
        let cell_index = world_cache_active_cell_indices[active_cell_id.x];

        let old_radiance = world_cache_radiance[cell_index];
        let new_radiance = world_cache_active_cells_new_radiance[active_cell_id.x];
        let sample_count = min(old_radiance.a + 1.0, WORLD_CACHE_MAX_TEMPORAL_SAMPLES);

        let blended_radiance = mix(old_radiance.rgb, new_radiance, 1.0 / sample_count);

        world_cache_radiance[cell_index] = vec4(blended_radiance, sample_count);
    }
}

fn sample_random_light_ris(world_position: vec3<f32>, world_normal: vec3<f32>, workgroup_id: vec2<u32>, rng: ptr<function, u32>) -> vec3<f32> {
    var workgroup_rng = (workgroup_id.x * 5782582u) + workgroup_id.y;
    let light_tile_start = rand_range_u(128u, &workgroup_rng) * 1024u;

    var weight_sum = 0.0;
    var selected_sample_radiance = vec3(0.0);
    var selected_sample_target_function = 0.0;
    var selected_sample_world_position = vec4(0.0);
    let mis_weight = 1.0 / f32(DIRECT_LIGHT_SAMPLE_COUNT);
    for (var i = 0u; i < DIRECT_LIGHT_SAMPLE_COUNT; i++) {
        let tile_sample = light_tile_start + rand_range_u(1024u, rng);
        let resolved_light_sample = unpack_resolved_light_sample(light_tile_resolved_samples[tile_sample], view.exposure);
        let light_contribution = calculate_resolved_light_contribution(resolved_light_sample, world_position, world_normal);

        let target_function = luminance(light_contribution.radiance);
        let resampling_weight = mis_weight * (target_function * light_contribution.inverse_pdf);

        weight_sum += resampling_weight;

        if rand_f(rng) < resampling_weight / weight_sum {
            selected_sample_radiance = light_contribution.radiance;
            selected_sample_target_function = target_function;
            selected_sample_world_position = resolved_light_sample.world_position;
        }
    }

    var unbiased_contribution_weight = 0.0;
    if all(selected_sample_radiance != vec3(0.0)) {
        let inverse_target_function = select(0.0, 1.0 / selected_sample_target_function, selected_sample_target_function > 0.0);
        unbiased_contribution_weight = weight_sum * inverse_target_function;

        unbiased_contribution_weight *= trace_light_visibility(world_position, selected_sample_world_position);
    }

    return selected_sample_radiance * unbiased_contribution_weight;
}
