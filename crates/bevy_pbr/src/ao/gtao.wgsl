#import bevy_pbr::mesh_view_types

@group(0) @binding(0) var prefiltered_depth: texture_2d<f32>;
@group(0) @binding(1) var normals: texture_2d<f32>;
@group(0) @binding(2) var hilbert_index: texture_2d<u32>;
@group(0) @binding(3) var ambient_occlusion: texture_storage_2d<r32uint, write>;
@group(0) @binding(4) var depth_differences: texture_storage_2d<r32uint, write>;
@group(0) @binding(5) var<uniform> globals: Globals;
@group(1) @binding(0) var point_clamp_sampler: sampler;
@group(1) @binding(1) var<uniform> view: View;

fn load_noise(pixel_coordinates: vec2<i32>) -> vec2<f32> {
    var index = textureLoad(hilbert_index, pixel_coordinates % 64, 0).x;

#ifdef TEMPORAL_NOISE
    index += 288u * (globals.frame_count % 64u);
#endif

    // R2 sequence - http://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences
    return fract(0.5 + f32(index) * vec2<f32>(0.75487766624669276005, 0.5698402909980532659114));
}

// Calculate differences in depth between neighbor pixels (later used by the spatial denoiser pass to preserve object edges)
fn calculate_neighboring_depth_differences(pixel_coordinates: vec2<i32>) -> f32 {
    // Sample the pixel's depth and 4 depths around it
    let uv = vec2<f32>(pixel_coordinates) / view.viewport.zw;
    let depths_upper_left = textureGather(0, prefiltered_depth, point_clamp_sampler, uv);
    let depths_bottom_right = textureGather(0, prefiltered_depth, point_clamp_sampler, uv, vec2<i32>(1i, 1i));
    let depth_center = depths_upper_left.y;
    let depth_left = depths_upper_left.x;
    let depth_top = depths_upper_left.z;
    let depth_bottom = depths_bottom_right.x;
    let depth_right = depths_bottom_right.z;

    // Calculate the depth differences (large differences represent object edges)
    var edge_info = vec4<f32>(depth_left, depth_right, depth_top, depth_bottom) - depth_center;
    let slope_left_right = (edge_info.y - edge_info.x) * 0.5;
    let slope_top_bottom = (edge_info.w - edge_info.z) * 0.5;
    let edge_info_slope_adjusted = edge_info + vec4<f32>(slope_left_right, -slope_left_right, slope_top_bottom, -slope_top_bottom);
    edge_info = min(abs(edge_info), abs(edge_info_slope_adjusted));
    edge_info = saturate(1.25 - edge_info / (depth_center * 0.011)); // TODO: ???

    // Pack the edge info into the texture
    let edge_info = vec4<u32>(pack4x8unorm(edge_info), 0u, 0u, 0u);
    textureStore(depth_differences, pixel_coordinates, edge_info);

    return depth_center;
}

fn screen_to_view_space(pixel_coordinates: vec2<i32>, view_depth: f32) -> vec3<f32> {
    return vec3<f32>(0.0); // TODO
}

fn gtao(pixel_coordinates: vec2<i32>, slice_count: u32, samples_per_slice_side: u32) {
    let view_depth = calculate_neighboring_depth_differences(pixel_coordinates);
    let view_normal = vec3<f32>(0.0); // TODO

    let noise = load_noise(pixel_coordinates);
    let noise_slice = noise.x;
    let noise_sample = noise.y;

    let view_position_center = screen_to_view_space(pixel_coordinates, view_depth);
    let view_vec = normalize(-view_position_center);

    var visiblity = 0.0;
    for (var slice = 0u; slice < slice_count; slice += 1u) {
        let pi = 3.1415926535897932384626433832795;
        let phi = (pi / f32(slice_count)) * f32(slice);
        let omega = vec2<f32>(cos(phi), sin(phi));

        let direction = vec3<f32>(omega.xy, 0.0);
        let orthographic_direction = direction - (dot(direction, view_vec) * view_vec)
        let axis = cross(direction, view_vec);
        let projected_normal = view_normal - axis * dot(view_normal, axis);
        let projected_normal_length = length(projected_normal);

        let sign_norm = sign(dot(orthographic_direction, projected_normal));
        let cos_norm = saturate(dot(projected_normal, view_vec) / projected_normal_length)
        let n = sign_norm * acos(cos_norm);

        for (var side = 0u; side < 2u; side += 1u) {
            var horizon_cos_center = -1;
            for (var sample = 0u; sample < samples_per_slice_side; sample += 1u) {
                // TODO
            }

            let horizon = n + clamp((-1.0 + 2.0 * f32(side)) * acos(horizon_cos_center) - n, -pi / 2.0, pi / 2.0);
            visiblity += projected_normal_length * (cos_norm + 2.0 * horizon * sin(n) - cos(2.0 * h - n)) / 4.0;
        }
    }
    visiblity /= f32(slice_count);
}

// TODO: Replace below when shader defines can hold values

@compute
@workgroup_size(8, 8, 1)
fn gtao_low(@builtin(global_invocation_id) global_id: vec3<u32>) {
    gtao(vec2<i32>(global_id.xy), 1u, 2u); // 4 spp (1 * (2 * 2)), plus optional temporal samples
}

@compute
@workgroup_size(8, 8, 1)
fn gtao_medium(@builtin(global_invocation_id) global_id: vec3<u32>) {
    gtao(vec2<i32>(global_id.xy), 2u, 2u); // 8 spp (2 * (2 * 2)), plus optional temporal samples
}

@compute
@workgroup_size(8, 8, 1)
fn gtao_high(@builtin(global_invocation_id) global_id: vec3<u32>) {
    gtao(vec2<i32>(global_id.xy), 3u, 3u); // 18 spp (3 * (3 * 2)), plus optional temporal samples
}

@compute
@workgroup_size(8, 8, 1)
fn gtao_ultra(@builtin(global_invocation_id) global_id: vec3<u32>) {
    gtao(vec2<i32>(global_id.xy), 9u, 3u); // 54 spp (9 * (3 * 2)), plus optional temporal samples
}
