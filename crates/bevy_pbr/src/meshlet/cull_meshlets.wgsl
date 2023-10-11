#import bevy_pbr::meshlet_bindings

@compute(8, 8, 1)
fn cull_meshlets(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let instanced_meshlet_id = global_id.x;
    if instanced_meshlet_id <= arrayLength(&instanced_meshlet_meshlet_indices) { return; }

    let meshlet_index = instanced_meshlet_meshlet_indices[instanced_meshlet_id];
    let meshlet = meshlets[meshlet_index];

    let frustum_culled = false; // TODO

    if !frustum_culled {
        let meshlet_index_count = meshlet.triangle_count * 3u;
        let draw_index_buffer_start = atomicAdd(&draw_command_buffer.index_count, meshlet_index_count);
        for (var offset = 0u; offset < meshlet_index_count; offset++) {
            // TODO: Mask off meshlet_index to get the right part of the packed value
            let meshlet_index = meshlet_indices[meshlet.indices_index / 4];
            // TODO: Pack instanced_meshlet_id and meshlet index into a u32 (24, 8)
            draw_index_buffer[draw_index_buffer_start + offset] = 0;
        }
    }
}
