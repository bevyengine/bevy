#define_import_path bevy_core_pipeline::oit

#import bevy_pbr::mesh_view_bindings::{view, oit_nodes, oit_heads, oit_atomic_counter, oit_settings}
#import bevy_pbr::mesh_view_types::OitFragmentNode
#import bevy_pbr::prepass_utils

#ifdef OIT_ENABLED
// Add the fragment to the oit buffer
fn oit_draw(position: vec4f, color: vec4f) {
#ifdef DEPTH_PREPASS
    if position.z < prepass_utils::prepass_depth(position, 0u) {
        return;
    }
#endif
    // Don't add fully transparent fragments to the list
    // because we don't want to have to sort them in the resolve pass
    if color.a < oit_settings.alpha_threshold {
        return;
    }
    // get the index of the current fragment relative to the screen size
    let screen_index = u32(floor(position.x) + floor(position.y) * view.viewport.z);
    // get the size of oit_nodes. It's screen_size * fragments_per_pixel_average
    let buffer_size = u32(view.viewport.z * view.viewport.w * oit_settings.fragments_per_pixel_average);

    var new_node_index = atomicAdd(&oit_atomic_counter, 1u);
    // exit early if we've reached the maximum amount of fragments nodes
    if new_node_index >= buffer_size {
        // TODO for tail blending we should return the color here
        return;
    }

    var node: OitFragmentNode;
    // In `oit_heads` buffer, index starts from 1, end sentinel is 0 so that we can avoid writing `u32::MAX` from CPU. wgpu guarantees buffers are zero-initialized.
    node.next = atomicExchange(&oit_heads[screen_index], new_node_index + 1u) - 1u;
    node.color = bevy_pbr::rgb9e5::vec3_to_rgb9e5_(color.rgb);
    node.depth_alpha = pack_24bit_depth_8bit_alpha(position.z, color.a);
    oit_nodes[new_node_index] = node;
}
#endif // OIT_ENABLED

// The packing scheme puts depth in the higher bits so that
//    depth(a) < depth(b) <=> packed(a) < packed(b)
// irregardless of alpha(a) and alpha(b)
// The property is used to optimize the resolve step
fn pack_24bit_depth_8bit_alpha(depth: f32, alpha: f32) -> u32 {
    let depth_bits = u32(saturate(depth) * f32(0xFFFFFFu) + 0.5);
    let alpha_bits = u32(saturate(alpha) * f32(0xFFu) + 0.5);
    return (depth_bits << 8u) | alpha_bits;
}

fn unpack_24bit_depth_8bit_alpha(packed: u32) -> vec2<f32> {
    let depth_bits = packed >> 8u;
    let alpha_bits = packed & 0xFFu;
    return vec2(f32(depth_bits) / f32(0xFFFFFFu), f32(alpha_bits) / f32(0xFFu));
}
