#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read_write> layers: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> layer_ids: array<atomic<i32>>;

@group(1) @binding(0) var depth: texture_depth_2d;

// Contains all the colors and depth for this specific fragment
// - X is rgba packed with 8 bits per channel
// - Y is the depth
var<private> fragment_list: array<vec2<u32>, 32>;

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let buffer_size = i32(view.viewport.z * view.viewport.w);
    let screen_index = i32(floor(in.position.x) + floor(in.position.y) * view.viewport.z);

    let counter = atomicLoad(&layer_ids[screen_index]);
    if counter == 0 {
        reset_indices(screen_index);
        discard;
    } else {
        let result = sort(screen_index, buffer_size);
        reset_indices(screen_index);

        // Manually do depth testing.
        // This is necesary because early z doesn't seem to trigger in the transparent pass.
        // Once we have a per pixel linked list it should be done much earlier
        let d = textureLoad(depth, vec2<i32>(in.position.xy), 0);
        if d > result.depth {
            discard;
        }

        return result.color;
    }
}

// Resets all indices to 0.
// This means we don't have to clear the entire layers buffer
fn reset_indices(screen_index: i32) {
    atomicStore(&layer_ids[screen_index], 0);
    layers[screen_index] = vec2(0u);
}

struct SortResult {
    color: vec4f,
    depth: f32,
}

fn sort(screen_index: i32, buffer_size: i32) -> SortResult {
    var counter = atomicLoad(&layer_ids[screen_index]);

    // fill list
    for (var i = 0; i < counter; i += 1) {
        fragment_list[i] = layers[screen_index + buffer_size * i];
    }

    // bubble sort the list based on the depth
    for (var i = counter; i >= 0; i -= 1) {
        for (var j = 0; j < i; j += 1) {
            if bitcast<f32>(fragment_list[j].y) < bitcast<f32>(fragment_list[j + 1].y) {
                // swap
                let temp = fragment_list[j + 1];
                fragment_list[j + 1] = fragment_list[j];
                fragment_list[j] = temp;
            }
        }
    }

    // resolve blend
    var final_color = vec4(0.0);
    for (var i = 0; i <= counter; i += 1) {
        let color = unpack4x8unorm(fragment_list[i].r);
        var base_color = vec4(color.rgb * color.a, color.a);
        final_color = blend(final_color, base_color);
    }
    var result: SortResult;
    result.color = final_color;
    result.depth = bitcast<f32>(fragment_list[0].y);

    return result;
}

// OVER operator using premultiplied alpha
// see: https://en.wikipedia.org/wiki/Alpha_compositing
fn blend(color_a: vec4<f32>, color_b: vec4<f32>) -> vec4<f32> {
    let final_color = color_a.rgb + (1.0 - color_a.a) * color_b.rgb;
    let alpha = color_a.a + (1.0 - color_a.a) * color_b.a;
    return vec4(final_color.rgb, alpha);
}