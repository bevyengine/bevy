// 5x5 bilaterial filter (edge-preserving blur)
// https://people.csail.mit.edu/sparis/bf_course/course_notes.pdf

#import bevy_pbr::mesh_view_types

@group(0) @binding(0) var ambient_occlusion_noisy: texture_storage_2d<r32float, write>;
@group(0) @binding(1) var ambient_occlusion: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var depth_differences: texture_storage_2d<r32uint, write>;
@group(1) @binding(0) var point_clamp_sampler: sampler;
@group(1) @binding(1) var<uniform> view: View;

@compute
@workgroup_size(8, 8, 1)
fn denoise(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_coordinates = global_id.xy * vec2<u32>(2u, 1u);
}
