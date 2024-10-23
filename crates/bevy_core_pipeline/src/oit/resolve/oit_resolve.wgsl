#import bevy_render::view::View

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read_write> layers: array<vec2<u32>>;
@group(0) @binding(2) var<storage, read_write> layer_ids: array<atomic<i32>>;

@group(1) @binding(0) var depth: texture_depth_2d;

struct OitFragment {
    color: vec3<f32>,
    alpha: f32,
    depth: f32,
}
// Contains all the colors and depth for this specific fragment
var<private> fragment_list: array<OitFragment, #{LAYER_COUNT}>;

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

        // https://github.com/gfx-rs/wgpu/issues/4416
        if true {
            discard;
        }
        return vec4(0.0);
    } else {
        // Load depth for manual depth testing.
        // This is necessary because early z doesn't seem to trigger in the transparent pass.
        // This should be done during the draw pass so those fragments simply don't exist in the list,
        // but this requires a bigger refactor
        let d = textureLoad(depth, vec2<i32>(in.position.xy), 0);
        let result = sort(screen_index, buffer_size, d);
        reset_indices(screen_index);

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

fn sort(screen_index: i32, buffer_size: i32, opaque_depth: f32) -> SortResult {
    var counter = atomicLoad(&layer_ids[screen_index]);

    // fill list
    for (var i = 0; i < counter; i += 1) {
        let fragment = layers[screen_index + buffer_size * i];
        // unpack color/alpha/depth
        let color = bevy_pbr::rgb9e5::rgb9e5_to_vec3_(fragment.x);
        let depth_alpha = bevy_core_pipeline::oit::unpack_24bit_depth_8bit_alpha(fragment.y);
        fragment_list[i].color = color;
        fragment_list[i].alpha = depth_alpha.y;
        fragment_list[i].depth = depth_alpha.x;
    }

    // bubble sort the list based on the depth
    for (var i = counter; i >= 0; i -= 1) {
        for (var j = 0; j < i; j += 1) {
            if fragment_list[j].depth < fragment_list[j + 1].depth {
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
        // depth testing
        // This needs to happen here because we can only stop iterating if the fragment is
        // occluded by something opaque and the fragments need to be sorted first
        if fragment_list[i].depth < opaque_depth {
            break;
        }
        let color = fragment_list[i].color;
        let alpha = fragment_list[i].alpha;
        var base_color = vec4(color.rgb * alpha, alpha);
        final_color = blend(final_color, base_color);
        if final_color.a == 1.0 {
            break;
        }
    }
    var result: SortResult;
    result.color = final_color;
    result.depth = fragment_list[0].depth;

    return result;
}

// OVER operator using premultiplied alpha
// see: https://en.wikipedia.org/wiki/Alpha_compositing
fn blend(color_a: vec4<f32>, color_b: vec4<f32>) -> vec4<f32> {
    let final_color = color_a.rgb + (1.0 - color_a.a) * color_b.rgb;
    let alpha = color_a.a + (1.0 - color_a.a) * color_b.a;
    return vec4(final_color.rgb, alpha);
}
