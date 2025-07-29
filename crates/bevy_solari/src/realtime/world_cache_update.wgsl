#import bevy_pbr::utils::sample_cosine_hemisphere
#import bevy_solari::sampling::sample_random_light
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, RAY_T_MIN, RAY_T_MAX}
#import bevy_solari::world_cache::{query_world_cache, world_cache_active_cells_count, world_cache_active_cell_indices, world_cache_geometry_data, world_cache_radiance, world_cache_active_cells_new_radiance}

struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

@compute
@workgroup_size(1024, 1, 1)
fn sample_radiance(@builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    if active_cell_id.x < world_cache_active_cells_count {
        let cell_index = world_cache_active_cell_indices[active_cell_id.x];
        let geometry_data = world_cache_geometry_data[cell_index];

        var rng = cell_index + constants.frame_index;

        let direct_lighting = sample_random_light(geometry_data.world_position, geometry_data.world_normal, &rng);
        var new_radiance = direct_lighting.radiance * direct_lighting.inverse_pdf;

#ifndef NO_MULTIBOUNCE
        let ray_direction = sample_cosine_hemisphere(geometry_data.world_normal, &rng);
        let ray_hit = trace_ray(geometry_data.world_position, ray_direction, RAY_T_MIN, RAY_T_MAX, RAY_FLAG_NONE);
        if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
            let ray_hit = resolve_ray_hit_full(ray_hit);
            new_radiance += ray_hit.material.base_color * query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal);
        }
#endif

        world_cache_active_cells_new_radiance[active_cell_id.x] = new_radiance;
    }
}

@compute
@workgroup_size(1024, 1, 1)
fn blend_new_samples(@builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    if active_cell_id.x < world_cache_active_cells_count {
        let cell_index = world_cache_active_cell_indices[active_cell_id.x];

        let old_radiance = world_cache_radiance[cell_index];
        let new_radiance = world_cache_active_cells_new_radiance[active_cell_id.x];

        var alpha = 0.1;
        if old_radiance.a == 0.0 {
            alpha = 1.0;
        }

        let blended_radiance = mix(old_radiance.rgb, new_radiance, alpha);

        world_cache_radiance[cell_index] = vec4(blended_radiance, 1.0);
    }
}
