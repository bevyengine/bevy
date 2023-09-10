#import bevy_solari::scene_bindings uniforms, map_ray_hit
#import bevy_solari::global_illumination::view_bindings view, depth_buffer, screen_probes
#import bevy_solari::world_cache::query query_world_cache
#import bevy_solari::utils rand_f, rand_vec2f, trace_ray, depth_to_world_position
#import bevy_pbr::utils octahedral_decode

@compute @workgroup_size(8, 8, 1)
fn trace_screen_probes(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(local_invocation_index) local_index: u32,
) {
    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    let frame_index = uniforms.frame_count * 5782582u;
    var rng = pixel_index + frame_index;
    var rng2 = frame_index;

    let probe_thread_id_offset = vec2<u32>(rand_vec2f(&rng2) * 7.0);
    let probe_thread_id = global_id.xy - local_id.xy + probe_thread_id_offset;
    let probe_pixel_depth = textureLoad(depth_buffer, probe_thread_id, 0i); // TODO: probe_thread_id may be off-screen
    if probe_pixel_depth == 0.0 {
        textureStore(screen_probes, global_id.xy, vec4(0.0, 0.0, 0.0, 1.0));
        return;
    }
    let probe_pixel_uv = (vec2<f32>(probe_thread_id) + 0.5) / view.viewport.zw;
    let probe_pixel_world_position = depth_to_world_position(probe_pixel_depth, probe_pixel_uv);

    let octahedral_pixel_center = vec2<f32>(local_id.xy) + rand_vec2f(&rng);
    let octahedral_pixel_uv = octahedral_pixel_center / 8.0;
    let octahedral_normal = octahedral_decode(octahedral_pixel_uv);

    var color = vec3(0.0);
    let ray_hit = trace_ray(probe_pixel_world_position, octahedral_normal, 0.001);
    if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
        let ray_hit = map_ray_hit(ray_hit);
        color = ray_hit.material.base_color * query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal);
    }

    textureStore(screen_probes, global_id.xy, vec4(color, 1.0));
}
