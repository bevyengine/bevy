#import bevy_solari::scene_bindings
#import bevy_solari::view_bindings
#import bevy_solari::utils
#import bevy_solari::world_cache::bindings
#import bevy_solari::world_cache::query

var<workgroup> probe_g_pixel: vec4<u32>;
var<workgroup> probe_pixel_uv: vec2<f32>;

@compute @workgroup_size(8, 8, 1)
fn update_screen_probes(
    @builtin(global_invocation_id) global_id: vec3<u32>,
    @builtin(local_invocation_id) local_id: vec3<u32>,
    @builtin(local_invocation_index) local_index: u32,
) {
    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    let frame_index = uniforms.frame_count * 5782582u;
    var rng = pixel_index + frame_index;
    var rng2 = frame_index;

    let probe_thread_index = u32(floor(rand_f(&rng2) * 63.0));
    if local_index == probe_thread_index {
        probe_g_pixel = textureLoad(g_buffer, global_id.xy); // TODO: may not exist
        probe_pixel_uv = (vec2<f32>(global_id.xy) + 0.5) / view.viewport.zw;
    }
    workgroupBarrier();
    let probe_depth = decode_g_buffer_depth(probe_g_pixel);
    if probe_depth < 0.0 {
        return;
    }

    let octahedral_pixel_center = vec2<f32>(local_id.xy) + rand_vec2(&rng);
    let octahedral_pixel_uv = octahedral_pixel_center / 8.0;
    let octahedral_normal = octahedral_decode(octahedral_pixel_uv);

    var color = vec3(0.0);
    let ray_origin = depth_to_world_position(probe_depth, probe_pixel_uv);
    let ray_hit = trace_ray(ray_origin, octahedral_normal, 0.001);
    if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
        let ray_hit = map_ray_hit(ray_hit);
        color = ray_hit.material.base_color * query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal);
    }

    textureStore(screen_probes_unfiltered, global_id.xy, vec4(color, 1.0));
}
