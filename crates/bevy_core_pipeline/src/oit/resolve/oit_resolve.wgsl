#import bevy_render::view::View
#import bevy_pbr::mesh_view_types::{OitFragmentNode, OrderIndependentTransparencySettings}

@group(0) @binding(0) var<uniform> view: View;
@group(0) @binding(1) var<storage, read> nodes: array<OitFragmentNode>;
@group(0) @binding(2) var<storage, read_write> heads: array<u32>; // No need to be atomic
@group(0) @binding(3) var<storage, read_write> atomic_counter: u32; // No need to be atomic

#ifndef DEPTH_PREPASS
@group(1) @binding(0) var depth: texture_depth_2d;
#endif

struct OitFragment {
    color: vec3<f32>,
    alpha: f32,
    depth: f32,
}

struct FullscreenVertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

const LINKED_LIST_END_SENTINEL: u32 = 0xFFFFFFFFu;
const SORTED_FRAGMENT_MAX_COUNT: u32 = #{SORTED_FRAGMENT_MAX_COUNT};

@fragment
fn fragment(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    atomic_counter = 0u;
    let screen_index = u32(floor(in.position.x) + floor(in.position.y) * view.viewport.z);

    let head = heads[screen_index] - 1u;
    if head == LINKED_LIST_END_SENTINEL {
        // https://github.com/gfx-rs/wgpu/issues/4416
        if true {
            discard;
        }
        return vec4(0.0);
    } else {
#ifndef DEPTH_PREPASS
        // If depth prepass is disabled, load depth for manual depth testing.
        // This is necessary because early z doesn't seem to trigger in the transparent pass.
        // This should be done during the draw pass so those fragments simply don't exist in the list,
        // but this requires a bigger refactor
        let d = textureLoad(depth, vec2<i32>(in.position.xy), 0);
#else
        let d = 0.0;
#endif
        let color = resolve(head, d);
        heads[screen_index] = 0u; // LINKED_LIST_END_SENTINEL + 1u;
        return color;
    }
}

fn resolve(head: u32, opaque_depth: f32) -> vec4<f32> {
    // Contains all the colors and depth for this specific fragment
    // Fragments are sorted from front to back (depth values are in descending order)
    // This should make insertion sort slightly faster
    // because transparent pass sorts objects so the linked list iteration is usually in descending order.
    var fragment_list: array<OitFragment, SORTED_FRAGMENT_MAX_COUNT>;
    var final_color = vec4<f32>(0.0);

    // fill list
    var current_node = head;
    var sorted_frag_count = 0u;
    while current_node != LINKED_LIST_END_SENTINEL {
        let fragment_node = nodes[current_node];
        // unpack color, alpha, depth
        let color = bevy_pbr::rgb9e5::rgb9e5_to_vec3_(fragment_node.color);
        let depth_alpha = bevy_core_pipeline::oit::unpack_24bit_depth_8bit_alpha(fragment_node.depth_alpha);
        current_node = fragment_node.next;

#ifndef DEPTH_PREPASS
        // depth testing
        if depth_alpha.x < opaque_depth {
            continue;
        }
#endif

        if sorted_frag_count < SORTED_FRAGMENT_MAX_COUNT {
            // There is still room in the sorted list.
            // Insert the fragment so that the list stay sorted.
            var i = sorted_frag_count;
            for(; i > 0; i -= 1) {
                // short-circuit can't be used in for(;;;), https://github.com/gfx-rs/wgpu/issues/4394
                if depth_alpha.x > fragment_list[i - 1].depth {
                    fragment_list[i] = fragment_list[i - 1];
                } else {
                    break;
                }
            }
            fragment_list[i].color = color;
            fragment_list[i].alpha = depth_alpha.y;
            fragment_list[i].depth = depth_alpha.x;
            sorted_frag_count += 1;
        } else if fragment_list[0].depth > depth_alpha.x {
            // The fragment is farther than the nearest sorted one.
            // First, make room by blending the nearest fragment from the sorted list.
            // Then, insert the fragment in the sorted list.
            // This is an approximation.
            let nearest_color = fragment_list[0].color;
            let nearest_alpha = fragment_list[0].alpha;
            final_color = blend(final_color, vec4f(nearest_color * nearest_alpha, nearest_alpha));
            var i = 0u;
            for(; i < SORTED_FRAGMENT_MAX_COUNT - 1; i += 1) {
                // short-circuit can't be used in for(;;;), https://github.com/gfx-rs/wgpu/issues/4394
                if depth_alpha.x < fragment_list[i + 1].depth {
                    fragment_list[i] = fragment_list[i + 1];
                } else {
                    break;
                }
            }
            fragment_list[i].color = color;
            fragment_list[i].alpha = depth_alpha.y;
            fragment_list[i].depth = depth_alpha.x;
        } else {
            // The next fragment is nearer than any of the sorted ones.
            // Blend it early.
            // This is an approximation.
            final_color = blend(final_color, vec4f(color * depth_alpha.y, depth_alpha.y));
        }
    }

    // blend sorted fragments
    for (var i = 0u; i < sorted_frag_count; i += 1) {
        let color = fragment_list[i].color;
        let alpha = fragment_list[i].alpha;
        var base_color = vec4(color.rgb * alpha, alpha);
        final_color = blend(final_color, base_color);
        if final_color.a == 1.0 {
            break;
        }
    }

    return final_color;
}

// OVER operator using premultiplied alpha
// see: https://en.wikipedia.org/wiki/Alpha_compositing
fn blend(color_a: vec4<f32>, color_b: vec4<f32>) -> vec4<f32> {
    let final_color = color_a.rgb + (1.0 - color_a.a) * color_b.rgb;
    let alpha = color_a.a + (1.0 - color_a.a) * color_b.a;
    return vec4(final_color.rgb, alpha);
}
