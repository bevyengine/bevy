#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::{rand_f, rand_range_u, sample_cosine_hemisphere}
#import bevy_render::view::View
#import bevy_solari::sampling::{calculate_resolved_light_contribution, trace_light_visibility}
#import bevy_solari::scene_bindings::{ResolvedMaterial, trace_ray, resolve_ray_hit_full, RAY_T_MIN, RAY_T_MAX}
#import bevy_solari::realtime_bindings::{
    world_cache_active_cells_count,
    world_cache_active_cell_indices,
    world_cache_geometry_data,
    world_cache_radiance,
    world_cache_active_cells_new_radiance,
    view,
    constants,
}
#import bevy_solari::world_cache::{
    query_world_cache_radiance,
    query_world_cache_lights, 
    evaluate_lighting_from_cache,
    write_world_cache_light,
    WORLD_CACHE_MAX_TEMPORAL_SAMPLES,
}

const MAX_GI_RAY_DISTANCE: f32 = 4.0;

@compute @workgroup_size(64, 1, 1)
fn sample_radiance(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(global_invocation_id) active_cell_id: vec3<u32>) {
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
        let direct_lighting = evaluate_lighting_from_cache(&rng, cell, geometry_data.world_position, geometry_data.world_normal, geometry_data.world_normal, material, view.exposure);
        write_world_cache_light(&rng, direct_lighting, geometry_data.world_position, geometry_data.world_normal, view.world_position, view.exposure);
        var new_radiance = direct_lighting.radiance;

#ifndef NO_MULTIBOUNCE
        let ray_direction = sample_cosine_hemisphere(geometry_data.world_normal, &rng);
        let ray_hit = trace_ray(geometry_data.world_position, ray_direction, RAY_T_MIN, MAX_GI_RAY_DISTANCE, RAY_FLAG_NONE);
        if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
            let ray_hit = resolve_ray_hit_full(ray_hit);
            new_radiance += ray_hit.material.base_color * query_world_cache_radiance(&rng, ray_hit.world_position, ray_hit.geometric_world_normal, view.world_position);
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
        let sample_count = min(old_radiance.a + 1.0, WORLD_CACHE_MAX_TEMPORAL_SAMPLES);

        let blended_radiance = mix(old_radiance.rgb, new_radiance, 1.0 / sample_count);

        world_cache_radiance[cell_index] = vec4(blended_radiance, sample_count);
    }
}
