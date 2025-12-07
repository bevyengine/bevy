#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::{rand_f, rand_range_u, sample_cosine_hemisphere}
#import bevy_render::view::View
#import bevy_solari::sampling::{calculate_resolved_light_contribution, trace_light_visibility}
#import bevy_solari::scene_bindings::{ResolvedMaterial, trace_ray, resolve_ray_hit_full, RAY_T_MIN, RAY_T_MAX}
#import bevy_solari::realtime_bindings::{
    world_cache_checksums,
    world_cache_active_cells_count,
    world_cache_active_cell_indices,
    world_cache_life,
    world_cache_geometry_data,
    world_cache_radiance,
    world_cache_light_data, 
    world_cache_light_data_new_lights, 
    world_cache_luminance_deltas,
    world_cache_active_cells_new_radiance,
    view,
    constants,
    WORLD_CACHE_CELL_LIGHT_COUNT,
    WorldCacheSingleLightData, 
}
#import bevy_solari::world_cache::{
    query_world_cache_radiance,
    query_world_cache_lights, 
    evaluate_lighting_from_cache,
    write_world_cache_light,
    WORLD_CACHE_MAX_TEMPORAL_SAMPLES,
    WORLD_CACHE_EMPTY_CELL,
    WORLD_CACHE_MAX_GI_RAY_DISTANCE,
}

#ifdef WORLD_CACHE_UPDATE_LIGHTS

@compute @workgroup_size(64, 1, 1)
fn update_lights(@builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    if active_cell_id.x < world_cache_active_cells_count {        
        let cell_index = world_cache_active_cell_indices[active_cell_id.x];
        var rng = cell_index + constants.frame_index;

        let old_data = world_cache_light_data[cell_index];
        let new_data = world_cache_light_data_new_lights[cell_index];
        let new_count = min(WORLD_CACHE_CELL_LIGHT_COUNT, new_data.visible_light_count);
        world_cache_light_data_new_lights[cell_index].visible_light_count = 0u;
        var out_i = 0u;
        var out_lights: array<WorldCacheSingleLightData, WORLD_CACHE_CELL_LIGHT_COUNT>;

        for (var i = 0u; i < new_count; i++) {
            let data = new_data.visible_lights[i];
            world_cache_light_data_new_lights[cell_index].visible_lights[i] = WorldCacheSingleLightData(0, 0.0);
            if data.weight == 0.0 { 
                break; 
            }

            var exist_index = 0u;
            if is_light_in_array(out_lights, out_i, data.light, &exist_index) {
                out_lights[exist_index].weight = max(out_lights[exist_index].weight, data.weight);
            } else {
                out_lights[out_i] = data;
                out_i++;
            }
        }
        for (var i = 0u; i < old_data.visible_light_count && out_i < WORLD_CACHE_CELL_LIGHT_COUNT; i++) {
            var exist_index = 0u;
            if is_light_in_array(out_lights, out_i, old_data.visible_lights[i].light, &exist_index) {
                out_lights[exist_index].weight = max(out_lights[exist_index].weight, old_data.visible_lights[i].weight);
            } else {
                out_lights[out_i] = old_data.visible_lights[i];
                out_i++;
            }
        }
        world_cache_light_data[cell_index].visible_light_count = out_i;
        world_cache_light_data[cell_index].visible_lights = out_lights;
    }
}

fn is_light_in_array(arr: array<WorldCacheSingleLightData, WORLD_CACHE_CELL_LIGHT_COUNT>, len: u32, light: u32, out_index: ptr<function, u32>) -> bool {
    var found: bool = false;
    for (var i = 0u; i < WORLD_CACHE_CELL_LIGHT_COUNT; i++) {
        let found_here = arr[i].light == light && i < len;
        *out_index = select(*out_index, i, found_here);
        found |= found_here;
    }
    return found;
}

#else

@compute @workgroup_size(64, 1, 1)
fn sample_radiance(@builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    if active_cell_id.x < world_cache_active_cells_count {
        let cell_index = world_cache_active_cell_indices[active_cell_id.x];
        let geometry_data = world_cache_geometry_data[cell_index];
        var rng = cell_index + constants.frame_index;

        // TODO: Initialize newly active cells with data from an adjacent LOD
    
        var material: ResolvedMaterial;
        material.base_color = vec3(1.0);
        material.emissive = vec3(0.0);
        material.reflectance = vec3(0.0);
        material.perceptual_roughness = 1.0;
        material.roughness = 1.0;
        material.metallic = 0.0;

        let cell = query_world_cache_lights(&rng, geometry_data.world_position, geometry_data.world_normal, view.world_position);
        let cell_life = atomicLoad(&world_cache_life[cell_index]);
        let direct_lighting = evaluate_lighting_from_cache(&rng, cell, geometry_data.world_position, geometry_data.world_normal, geometry_data.world_normal, material, view.exposure);
        write_world_cache_light(&rng, direct_lighting, geometry_data.world_position, geometry_data.world_normal, view.world_position, cell_life, view.exposure);
        var new_radiance = direct_lighting.radiance * direct_lighting.inverse_pdf;

#ifndef NO_MULTIBOUNCE
        let ray_direction = sample_cosine_hemisphere(geometry_data.world_normal, &rng);
        let ray_hit = trace_ray(geometry_data.world_position, ray_direction, RAY_T_MIN, WORLD_CACHE_MAX_GI_RAY_DISTANCE, RAY_FLAG_NONE);
        if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
            let ray_hit = resolve_ray_hit_full(ray_hit);
            new_radiance += ray_hit.material.base_color * query_world_cache_radiance(&rng, ray_hit.world_position, ray_hit.geometric_world_normal, view.world_position, cell_life);
        }
#endif

        world_cache_active_cells_new_radiance[active_cell_id.x] = new_radiance;
    }
}

@compute @workgroup_size(64, 1, 1)
fn blend_new_samples(@builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    if active_cell_id.x < world_cache_active_cells_count {
        let cell_index = world_cache_active_cell_indices[active_cell_id.x];

        let old_radiance = world_cache_radiance[cell_index];
        let new_radiance = world_cache_active_cells_new_radiance[active_cell_id.x];
        let luminance_delta = world_cache_luminance_deltas[cell_index];

        // https://bsky.app/profile/gboisse.bsky.social/post/3m5blga3ftk2a
        let sample_count = min(old_radiance.a + 1.0, WORLD_CACHE_MAX_TEMPORAL_SAMPLES);
        let alpha = abs(luminance_delta) / max(luminance(old_radiance.rgb), 0.001);
        let max_sample_count = mix(WORLD_CACHE_MAX_TEMPORAL_SAMPLES, 1.0, pow(saturate(alpha), 1.0 / 8.0));
        let blend_amount = 1.0 / min(sample_count, max_sample_count);

        let blended_radiance = mix(old_radiance.rgb, new_radiance, blend_amount);
        let blended_luminance_delta = mix(luminance_delta, luminance(blended_radiance) - luminance(old_radiance.rgb), 1.0 / 8.0);

        world_cache_radiance[cell_index] = vec4(blended_radiance, sample_count);
        world_cache_luminance_deltas[cell_index] = blended_luminance_delta;
    }
}

#endif
