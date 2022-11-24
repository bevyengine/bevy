#import bevy_pbr::ao_settings
#import bevy_pbr::mesh_view_types

@group(0) @binding(0) var prefiltered_depth: texture_2d<f32>;
@group(0) @binding(1) var normals: texture_2d<f32>;
@group(0) @binding(2) var hilbert_index: texture_2d<u32>;
@group(0) @binding(3) var ambient_occlusion: texture_storage_2d<r32uint, write>;
@group(0) @binding(4) var depth_differences: texture_storage_2d<r32uint, write>;
@group(0) @binding(5) var<uniform> globals: Globals;
@group(1) @binding(0) var point_clamp_sampler: sampler;
@group(1) @binding(1) var<uniform> ao_settings: AmbientOcclusionSettings;
@group(1) @binding(2) var<uniform> view: View;

fn load_noise(pixel_coordinates: vec2<i32>) -> vec2<f32> {
    var index = textureLoad(hilbert_index, pixel_coordinates % 64, 0).x;

#ifdef TEMPORAL_NOISE
    index += 288u * (globals.frame_count % 64u);
#endif

    // R2 sequence - http://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences
    return fract(0.5 + f32(index) * vec2<f32>(0.75487766624669276005, 0.5698402909980532659114));
}

// Calculate differences in depth between neighbor pixels (later used by the spatial denoiser pass to preserve object edges)
fn calculate_neighboring_depth_differences(pixel_coordinates: vec2<i32>) {
    // Sample the pixel's depth and 4 depths around it
    let uv = vec2<f32>(pixel_coordinates) / view.viewport.zw;
    let depths_upper_left = textureGather(0, prefiltered_depth, point_clamp_sampler, uv);
    let depths_bottom_right = textureGather(0, prefiltered_depth, point_clamp_sampler, uv, vec2<i32>(1i, 1i));
    let depth_center = depths_upper_left.y;
    let depth_left = depths_upper_left.x;
    let depth_top = depths_upper_left.z;
    let depth_bottom = depths_bottom_right.x;
    let depth_right = depths_bottom_right.z;

    // Calculate the depth differences (which represent object edges when the difference is large)
    var edge_info = vec4<f32>(depth_left, depth_right, depth_top, depth_bottom) - depth_center;
    let slope_left_right = (edge_info.y - edge_info.x) * 0.5;
    let slope_top_bottom = (edge_info.w - edge_info.z) * 0.5;
    let edge_info_slope_adjusted = edge_info + vec4<f32>(slope_left_right, -slope_left_right, slope_top_bottom, -slope_top_bottom);
    edge_info = min(abs(edge_info), abs(edge_info_slope_adjusted));
    edge_info = saturate(1.25 - edge_info / (depth_center * 0.011)); // TODO: ???

    // Pack the edge info into the texture
    let edge_info = vec4<u32>(pack4x8unorm(edge_info), 0u, 0u, 0u);
    textureStore(depth_differences, pixel_coordinates, edge_info);
}

@compute
@workgroup_size(8, 8, 1)
fn gtao(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let pixel_coordinates = vec2<i32>(global_id.xy);

    calculate_neighboring_depth_differences(pixel_coordinates);

    let noise = load_noise(pixel_coordinates);
    let noise_slice = noise.x;
    let noise_sample = noise.y;
}
