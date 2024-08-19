#define_import_path bevy_core_pipeline::oit

#import bevy_pbr::mesh_view_bindings::{view, oit_layers, oit_layer_ids}

// TODO use push constant or a uniform or something for this
const layer_count: i32 = 8;

// TODO I don't think this def should be necessary here but it doesn't work if I remove it.
#ifdef OIT_ENABLED
// Add the fragment to the oit buffer
fn oit_draw(position: vec4f, color: vec4f) -> vec4f {
    // get the index of the current fragment relative to the screen size
    let screen_index = i32(floor(position.x) + floor(position.y) * view.viewport.z);
    // get the size of the buffer.
    // It's always the size of the screen
    let buffer_size = i32(view.viewport.z * view.viewport.w);

    // gets the layer index of the current fragment
    var layer_id = atomicAdd(&oit_layer_ids[screen_index], 1);
    // exit early if we've reached the maximum amount of fragments per layer
    if layer_id >= layer_count {
        // force to store the layer_count to make sure we don't
        // accidentally increase the index above the maximum value
        atomicStore(&oit_layer_ids[screen_index], layer_count);
        // TODO for tail blending we should return the color here
        discard;
    }

    // get the layer_index from the screen
    let layer_index = screen_index + layer_id * buffer_size;
    // TODO consider a different packing strategy,
    // this loses a lot of color accuracy
    let packed_color = pack4x8unorm(color);
    let depth = bitcast<u32>(position.z);
    oit_layers[layer_index] = vec2(packed_color, depth);
    discard;
}
#endif // OIT_ENABLED