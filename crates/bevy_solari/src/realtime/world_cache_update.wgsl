// TODO: Import
struct WorldCacheGeometryData {
    position: vec3<f32>,
    padding1: u32,
    normal: vec3<f32>,
    padding2: u32
}

@group(1) @binding(16) var<storage, read_write> world_cache_radiance: array<vec4<f32>, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(17) var<storage, read_write> world_cache_geometry_data: array<WorldCacheGeometryData, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(18) var<storage, read_write> world_cache_active_cells_new_radiance: array<vec3<f32>, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(21) var<storage, read_write> world_cache_active_cell_indices: array<u32, #{WORLD_CACHE_SIZE}>;
@group(1) @binding(22) var<storage, read_write> world_cache_active_cells_count: u32;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

@compute
@workgroup_size(1024, 1, 1)
fn sample_radiance(@builtin(global_invocation_id) active_cell_id: vec3<u32>) {
    if active_cell_id.x < world_cache_active_cells_count {
        let cell_index = world_cache_active_cell_indices[active_cell_id.x];
        let cell_data = world_cache_geometry_data[cell_index];

        var rng = cell_index + constants.frame_index;

        var radiance = vec3(0.0);

        // radiance += sample_direct_lighting(cell_data.position, cell_data.normal, &rng);

        // let ray_direction = sample_cosine_hemisphere(cell_data.normal, &rng);
        // let ray_hit = trace_ray(cell_data.position, ray_direction, 0.001, 1000.0);
        // if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
        //     let ray_hit = map_ray_hit(ray_hit);
        //     radiance += ray_hit.material.base_color * query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal);
        // }

        world_cache_active_cells_new_radiance[active_cell_id.x] = radiance;
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
