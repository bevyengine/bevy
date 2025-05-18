#define_import_path bevy_core_pipeline::oit

#import bevy_pbr::mesh_view_bindings::{view, oit_layers, oit_layer_ids, oit_settings}

#ifdef OIT_ENABLED
// Add the fragment to the oit buffer
fn oit_draw(position: vec4f, color: vec4f) {
    // Don't add fully transparent fragments to the list
    // because we don't want to have to sort them in the resolve pass
    if color.a < oit_settings.alpha_threshold {
        return;
    }
    // get the index of the current fragment relative to the screen size
    let screen_index = i32(floor(position.x) + floor(position.y) * view.viewport.z);
    // get the size of the buffer.
    // It's always the size of the screen
    let buffer_size = i32(view.viewport.z * view.viewport.w);

    // gets the layer index of the current fragment
    var layer_id = atomicAdd(&oit_layer_ids[screen_index], 1);
    // exit early if we've reached the maximum amount of fragments per layer
    if layer_id >= oit_settings.layers_count {
        // force to store the oit_layers_count to make sure we don't
        // accidentally increase the index above the maximum value
        atomicStore(&oit_layer_ids[screen_index], oit_settings.layers_count);
        // TODO for tail blending we should return the color here
        return;
    }

    // get the layer_index from the screen
    let layer_index = screen_index + layer_id * buffer_size;
    let rgb9e5_color = bevy_pbr::rgb9e5::vec3_to_rgb9e5_(color.rgb);
    let depth_alpha = pack_24bit_depth_8bit_alpha(position.z, color.a);
    oit_layers[layer_index] = vec2(rgb9e5_color, depth_alpha);
}
#endif // OIT_ENABLED

fn pack_24bit_depth_8bit_alpha(depth: f32, alpha: f32) -> u32 {
    let depth_bits = u32(saturate(depth) * f32(0xFFFFFFu) + 0.5);
    let alpha_bits = u32(saturate(alpha) * f32(0xFFu) + 0.5);
    return (depth_bits & 0xFFFFFFu) | ((alpha_bits & 0xFFu) << 24u);
}

fn unpack_24bit_depth_8bit_alpha(packed: u32) -> vec2<f32> {
    let depth_bits = packed & 0xFFFFFFu;
    let alpha_bits = (packed >> 24u) & 0xFFu;
    return vec2(f32(depth_bits) / f32(0xFFFFFFu), f32(alpha_bits) / f32(0xFFu));
}
