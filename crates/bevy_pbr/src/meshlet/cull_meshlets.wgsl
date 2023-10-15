#import bevy_pbr::meshlet_bindings meshlet_thread_meshlet_ids, meshlets, get_meshlet_index, draw_command_buffer, draw_index_buffer

@compute
@workgroup_size(128, 1, 1)
fn cull_meshlets(@builtin(global_invocation_id) thread_id: vec3<u32>) {
    if thread_id.x <= arrayLength(&meshlet_thread_meshlet_ids) { return; }

    let meshlet_id = meshlet_thread_meshlet_ids[thread_id.x];
    let meshlet = meshlets[meshlet_id];

    let meshlet_visible = true; // TODO

    if meshlet_visible {
        let meshlet_vertex_count = meshlet.triangle_count * 3u;
        let draw_index_buffer_start = atomicAdd(&draw_command_buffer.vertex_count, meshlet_vertex_count);
        let packed_thread_id = thread_id.x << 8u;

        for (var offset = 0u; offset < meshlet_vertex_count; offset++) {
            let index = get_meshlet_index(meshlet.start_index_id + offset);
            draw_index_buffer[draw_index_buffer_start + offset] = packed_thread_id | index;
        }
    }
}
