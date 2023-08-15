// 3x3 bilaterial filter (edge-preserving blur)
// https://people.csail.mit.edu/sparis/bf_course/course_notes.pdf

// Note: Does not use the Gaussian kernel part of a typical bilateral blur
// From the paper: "use the information gathered on a neighborhood of 4 × 4 using a bilateral filter for
// reconstruction, using _uniform_ convolution weights"

// Note: The paper does a 4x4 (not quite centered) filter, offset by +/- 1 pixel every other frame
// XeGTAO does a 3x3 filter, on two pixels at a time per compute thread, applied twice
// We do a 3x3 filter, on 1 pixel per compute thread, applied once

#import bevy_render::view View

@group(0) @binding(0) var ambient_occlusion_noisy: texture_2d<f32>;
@group(0) @binding(1) var depth_differences: texture_2d<u32>;
@group(0) @binding(2) var ambient_occlusion: texture_storage_2d<r16float, write>;
@group(1) @binding(0) var point_clamp_sampler: sampler;
@group(1) @binding(1) var<uniform> view: View;

@compute
@workgroup_size(8, 8, 1)
fn spatial_denoise(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_coordinates = vec2<i32>(global_id.xy);
    let uv = vec2<f32>(pixel_coordinates) / view.viewport.zw;

    let edges0 = textureGather(0, depth_differences, point_clamp_sampler, uv);
    let edges1 = textureGather(0, depth_differences, point_clamp_sampler, uv, vec2<i32>(2i, 0i));
    let edges2 = textureGather(0, depth_differences, point_clamp_sampler, uv, vec2<i32>(1i, 2i));
    let visibility0 = textureGather(0, ambient_occlusion_noisy, point_clamp_sampler, uv);
    let visibility1 = textureGather(0, ambient_occlusion_noisy, point_clamp_sampler, uv, vec2<i32>(2i, 0i));
    let visibility2 = textureGather(0, ambient_occlusion_noisy, point_clamp_sampler, uv, vec2<i32>(0i, 2i));
    let visibility3 = textureGather(0, ambient_occlusion_noisy, point_clamp_sampler, uv, vec2<i32>(2i, 2i));

    let left_edges = myunpack4x8unorm(edges0.x);
    let right_edges = myunpack4x8unorm(edges1.x);
    let top_edges = myunpack4x8unorm(edges0.z);
    let bottom_edges = myunpack4x8unorm(edges2.w);
    var center_edges = myunpack4x8unorm(edges0.y);
    center_edges *= vec4<f32>(left_edges.y, right_edges.x, top_edges.w, bottom_edges.z);

    let center_weight = 1.2;
    let left_weight = center_edges.x;
    let right_weight = center_edges.y;
    let top_weight = center_edges.z;
    let bottom_weight = center_edges.w;
    let top_left_weight = 0.425 * (top_weight * top_edges.x + left_weight * left_edges.z);
    let top_right_weight = 0.425 * (top_weight * top_edges.y + right_weight * right_edges.z);
    let bottom_left_weight = 0.425 * (bottom_weight * bottom_edges.x + left_weight * left_edges.w);
    let bottom_right_weight = 0.425 * (bottom_weight * bottom_edges.y + right_weight * right_edges.w);

    let center_visibility = visibility0.y;
    let left_visibility = visibility0.x;
    let right_visibility = visibility0.z;
    let top_visibility = visibility1.x;
    let bottom_visibility = visibility2.z;
    let top_left_visibility = visibility0.w;
    let top_right_visibility = visibility1.w;
    let bottom_left_visibility = visibility2.w;
    let bottom_right_visibility = visibility3.w;

    var sum = center_visibility;
    sum += left_visibility * left_weight;
    sum += right_visibility * right_weight;
    sum += top_visibility * top_weight;
    sum += bottom_visibility * bottom_weight;
    sum += top_left_visibility * top_left_weight;
    sum += top_right_visibility * top_right_weight;
    sum += bottom_left_visibility * bottom_left_weight;
    sum += bottom_right_visibility * bottom_right_weight;

    var sum_weight = center_weight;
    sum_weight += left_weight;
    sum_weight += right_weight;
    sum_weight += top_weight;
    sum_weight += bottom_weight;
    sum_weight += top_left_weight;
    sum_weight += top_right_weight;
    sum_weight += bottom_left_weight;
    sum_weight += bottom_right_weight;

    let denoised_visibility = sum / sum_weight;

    textureStore(ambient_occlusion, pixel_coordinates, vec4<f32>(denoised_visibility, 0.0, 0.0, 0.0));
}

// TODO: Remove this once https://github.com/gfx-rs/naga/pull/2353 lands in Bevy
fn myunpack4x8unorm(e: u32) -> vec4<f32> {
    return vec4<f32>(clamp(f32(e & 0xFFu) / 255.0, 0.0, 1.0),
        clamp(f32((e >> 8u) & 0xFFu) / 255.0, 0.0, 1.0),
        clamp(f32((e >> 16u) & 0xFFu) / 255.0, 0.0, 1.0),
        clamp(f32((e >> 24u) & 0xFFu) / 255.0, 0.0, 1.0));
}
