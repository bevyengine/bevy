#import bevy_solari::scene_bindings map_ray_hit, uniforms
#import bevy_solari::global_illumination::view_bindings view, depth_buffer, screen_probes_a, FIRST_RADIANCE_CASCADE_INTERVAL
#import bevy_solari::world_cache::query query_world_cache
#import bevy_solari::utils trace_ray, depth_to_world_position, rand_vec2f
#import bevy_pbr::utils octahedral_decode

@compute @workgroup_size(8, 8, 1)
fn trace_screen_probes(@builtin(global_invocation_id) global_id: vec3<u32>) {
    // Find the center texel of each probe tile for this thread (global_id.xy = texel coordinate, global_id.z = cascade)
    let probe_size = u32(exp2(f32(global_id.z) + 3.0));
    var probe_center_pixel_id = ((global_id.xy / probe_size) * probe_size) + (probe_size / 2u - 1u);
    probe_center_pixel_id = min(probe_center_pixel_id, vec2<u32>(view.viewport.zw) - 1u);

    // Reconstruct world position of the probe and early out if the probe is placed on a background pixel
    let probe_depth = textureLoad(depth_buffer, probe_center_pixel_id, 0i);
    if probe_depth == 0.0 {
        textureStore(screen_probes_a, global_id.xy, global_id.z, vec4(0.0, 0.0, 0.0, 1.0));
        return;
    }
    let probe_center_uv = (vec2<f32>(probe_center_pixel_id) + 0.5) / view.viewport.zw;
    let probe_world_position = depth_to_world_position(probe_depth, probe_center_uv);

    // Calculate world-space normal of the assigned probe texel for this thread
    var rng = uniforms.frame_count * 5782582u;
    let probe_cell_center = vec2<f32>(global_id.xy % probe_size) + rand_vec2f(&rng);
    let probe_cell_uv = probe_cell_center / f32(probe_size);
    let probe_cell_normal = octahedral_decode(probe_cell_uv);

    // Calculate radiance interval for this probe based on which cascade it's part of
    let i = f32(global_id.z);
    var radiance_interval_min = FIRST_RADIANCE_CASCADE_INTERVAL * (exp2(i) - 1.0);
    var radiance_interval_max = FIRST_RADIANCE_CASCADE_INTERVAL * (exp2(i + 1.0) - 1.0);
    if global_id.z == 0u {
        radiance_interval_min = 0.001;
    }

    // Trace radiance interval, query world cache for lighting at hit
    var color = vec4(0.0, 0.0, 0.0, 1.0);
    let ray_hit = trace_ray(probe_world_position, probe_cell_normal, radiance_interval_min, radiance_interval_max);
    if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
        let ray_hit = map_ray_hit(ray_hit);
        let hit_color = ray_hit.material.base_color * query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal);
        color = vec4(hit_color, 0.0);
    }

    // Store lighting and hit/no-hit in probe texel
    textureStore(screen_probes_a, global_id.xy, global_id.z, color);
}
