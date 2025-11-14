#import bevy_render::view::View
#import bevy_pbr::mesh_view_types::{OitFragmentNode, OrderIndependentTransparencySettings}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read> nodes: array<OitFragmentNode>;
@group(0) @binding(2) var<storage, read_write> headers: array<u32>;
@group(0) @binding(3) var<storage, read_write> atomic_counter: u32;

@group(1) @binding(0) var depth: texture_depth_2d;

struct OitFragment {
    color: vec3<f32>,
    alpha: f32,
    depth: f32,
}
// Contains all the colors and depth for this specific fragment
var<private> fragment_list: array<OitFragment, #{SORTED_FRAGMENT_MAX_COUNT}>;

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

const LINKED_LIST_END_SENTINEL: u32 = 0xFFFFFFFFu;

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    atomic_counter = 0u;
    let screen_index = u32(floor(in.position.x) + floor(in.position.y) * view.viewport.z);

    let header = headers[screen_index];
    if header == LINKED_LIST_END_SENTINEL {
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
        let result = resolve(header, d);
        headers[screen_index] = LINKED_LIST_END_SENTINEL;
        return result.color;
    }
}

struct SortResult {
    color: vec4f,
    depth: f32,
}

fn resolve(header: u32, opaque_depth: f32) -> SortResult {
    var final_color = vec4(0.0);

    // fill list
    var current_node = header;
    var sorted_frag_count = 0u;
    while current_node != LINKED_LIST_END_SENTINEL {
        let fragment_node = nodes[current_node];
        // unpack color/alpha/depth
        let color = bevy_pbr::rgb9e5::rgb9e5_to_vec3_(fragment_node.color);
        let depth_alpha = bevy_core_pipeline::oit::unpack_24bit_depth_8bit_alpha(fragment_node.depth_alpha);
        current_node = fragment_node.next;

        if sorted_frag_count < #{SORTED_FRAGMENT_MAX_COUNT} {
            // There is still room in the sorted list.
            // Insert the fragment so that the list stay sorted.
            var i = sorted_frag_count;
            for(; (i > 0) && (depth_alpha.x < fragment_list[i - 1].depth); i -= 1) {
                fragment_list[i] = fragment_list[i - 1];
            }
            fragment_list[i].color = color;
            fragment_list[i].alpha = depth_alpha.y;
            fragment_list[i].depth = depth_alpha.x;
            sorted_frag_count += 1;
        } else if fragment_list[0].depth < depth_alpha.x {
            // The fragment is closer than the farthest sorted one.
            // First, make room by blending the farthest fragment from the sorted list.
            // Then, insert the fragment in the sorted list.
            // This is an approximation.
            final_color = blend(vec4f(fragment_list[0].color * fragment_list[0].alpha, fragment_list[0].alpha), final_color);
            var i = 0u;
            for(; (i < #{SORTED_FRAGMENT_MAX_COUNT} - 1) && (fragment_list[i + 1].depth < depth_alpha.x); i += 1) {
               fragment_list[i] = fragment_list[i + 1];
            }
            fragment_list[i].color = color;
            fragment_list[i].alpha = depth_alpha.y;
            fragment_list[i].depth = depth_alpha.x;
        } else {
            // The next fragment is farther than any of the sorted ones.
            // Blend it early.
            // This is an approximation.
            final_color = blend(vec4f(color * depth_alpha.y, depth_alpha.y), final_color);
        }
    }

    // blend sorted fragments
    for (var i = 0u; i < sorted_frag_count; i += 1) {
        // depth testing
        // This needs to happen here because we can only stop iterating if the fragment is
        // occluded by something opaque and the fragments need to be sorted first
        if fragment_list[i].depth < opaque_depth {
            break;
        }
        let color = fragment_list[i].color;
        let alpha = fragment_list[i].alpha;
        var base_color = vec4(color.rgb * alpha, alpha);
        final_color = blend(base_color, final_color);
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
