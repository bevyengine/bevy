#import bevy_solari::scene_bindings
#import bevy_solari::view_bindings
#import bevy_solari::utils
#import bevy_solari::world_cache::bindings
#import bevy_solari::world_cache::query

@compute @workgroup_size(1024, 1, 1)
fn world_cache_sample_irradiance(@builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    if active_cell_id.x < world_cache_active_cells_count {
        let cell_index = world_cache_active_cell_indices[active_cell_id.x];
        let cell_data = world_cache_cell_data[cell_index];

        let frame_index = uniforms.frame_count * 5782582u;
        var rng = cell_index + frame_index;

        var irradiance = vec3(0.0);

        irradiance += sample_direct_lighting(cell_data.position, cell_data.normal, &rng);

        let ray_direction = sample_cosine_hemisphere(cell_data.normal, &rng);
        let ray_hit = trace_ray(cell_data.position, ray_direction, 0.001);
        if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
            let ray_hit = map_ray_hit(ray_hit);
            irradiance += ray_hit.material.base_color * query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal);
        }

        world_cache_active_cells_new_irradiance[active_cell_id.x] = irradiance;
    }
}

@compute @workgroup_size(1024, 1, 1)
fn world_cache_blend_new_samples(@builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    if active_cell_id.x < world_cache_active_cells_count {
        let cell_index = world_cache_active_cell_indices[active_cell_id.x];

        let old_irradiance = world_cache_irradiance[cell_index];
        let new_irradiance = world_cache_active_cells_new_irradiance[active_cell_id.x];

        var alpha = 0.1;
        if old_irradiance.a == 0.0 {
            alpha = 1.0;
        }

        let blended_irradiance = mix(old_irradiance.rgb, new_irradiance, alpha);

        world_cache_irradiance[cell_index] = vec4(blended_irradiance, 1.0);
    }
}
