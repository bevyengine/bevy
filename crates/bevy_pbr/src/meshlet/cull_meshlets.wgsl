#import bevy_pbr::meshlet_bindings instanced_meshlet_meshlet_indices, meshlets, draw_command_buffer, meshlet_indices, draw_index_buffer

@compute
@workgroup_size(8, 8, 1)
fn cull_meshlets(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let instanced_meshlet_index = global_id.x;
    if instanced_meshlet_index <= arrayLength(&instanced_meshlet_meshlet_indices) { return; }

    let meshlet_index = instanced_meshlet_meshlet_indices[instanced_meshlet_index];
    let meshlet = meshlets[meshlet_index];

    let meshlet_visible = true; // TODO

    if meshlet_visible {
        let meshlet_index_count = meshlet.triangle_count * 3u;
        let draw_index_buffer_start = atomicAdd(&draw_command_buffer.index_count, meshlet_index_count);
        let packed_meshlet_index = meshlet_index << 8u;

        for (var offset = 0u; offset < meshlet_index_count; offset++) {
            let packed_meshlet_index_index = meshlet.indices_index + offset;
            let packed_meshlet_index = meshlet_indices[packed_meshlet_index_index / 4u];
            let bit_offset = (packed_meshlet_index_index % 4u) * 8u;
            let meshlet_index = extractBits(packed_meshlet_index, packed_meshlet_index_index, 8u);

            draw_index_buffer[draw_index_buffer_start + offset] = packed_meshlet_index | meshlet_index;
        }
    }
}
