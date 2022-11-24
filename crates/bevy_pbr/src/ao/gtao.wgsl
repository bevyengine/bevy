#import bevy_pbr::mesh_view_types

@group(0) @binding(0) var prefiltered_depth: texture_2d<f32>;
@group(0) @binding(1) var normals: texture_2d<f32>;
@group(0) @binding(2) var hilbert_index: texture_2d<u32>;
@group(0) @binding(3) var ambient_occlusion: texture_storage_2d<r32uint, write>;
@group(0) @binding(4) var depth_differences: texture_storage_2d<r32uint, write>;
@group(0) @binding(5) var<uniform> globals: Globals;
@group(1) @binding(0) var point_clamp_sampler: sampler;
@group(1) @binding(1) var<uniform> view: View;

struct Noise {
    slice: u32,
    sample: u32,
};

fn load_noise(pixel_coordinates: vec2<i32>) -> Noise {
    var index = textureLoad(hilbert_index, pixel_coordinates % 64, 0).r;

#ifdef TEMPORAL_NOISE
    index += 288u * (globals.frame_count % 64u);
#endif

    // R2 sequence - http://extremelearning.com.au/unreasonable-effectiveness-of-quasirandom-sequences
    let n = fract(0.5 + f32(index) * vec2<f32>(0.75487766624669276005, 0.5698402909980532659114));

    var noise: Noise;
    noise.slice = u32(n.x);
    noise.sample = u32(n.y);
    return noise;
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

fn load_normal_view_space(uv: vec2<f32>) -> vec3<f32> {
    let normal = textureSampleLevel(normals, point_clamp_sampler, uv, 0.0);
    let normal = (normal * 2.0) - 1.0;
    return (view.view * normal).xyz; // Convert from world to view space
}

fn reconstruct_view_space_position(depth: f32, uv: vec2<f32>) -> vec3<f32> {
    let clip_xy = vec2<f32>(uv.x * 2.0 - 1.0, 1.0 - 2.0 * uv.y);
    let t = view.inverse_projection * vec4<f32>(clip_xy, depth, 1.0);
    let view_xyz = t.xyz / t.w;
    return view_xyz;
}

fn load_and_reconstruct_view_space_position(uv: vec2<f32>) -> vec3<f32> {
    let depth = textureSampleLevel(prefiltered_depth, point_clamp_sampler, uv, 0.0).r;
    return reconstruct_view_space_position(depth, uv);
}

fn gtao(pixel_coordinates: vec2<i32>, slice_count: u32, samples_per_slice_side: u32) {
    let pi = 3.1415926535897932384626433832795;
    let half_pi = pi / 2.0;

    var pixel_depth = calculate_neighboring_depth_differences(pixel_coordinates);
    pixel_depth *= 0.99999; // TODO: XeGTAO avoid precision artifacts, is needed?

    let uv = (vec2<f32>(pixel_coordinates) + 0.5) / view.viewport.zw;
    let pixel_position = reconstruct_view_space_position(pixel_depth, uv);
    let view_vec = normalize(-pixel_position);
    let pixel_normal = load_normal_view_space(uv);
    let noise = load_noise(pixel_coordinates);

    var visiblity = 0.0;
    for (var s = 0u; s < slice_count; s += 1u) {
        let slice = f32(s + noise.slice);
        let phi = (pi / f32(slice_count)) * slice;
        let omega = vec2<f32>(cos(phi), sin(phi));

        let direction = vec3<f32>(omega.xy, 0.0);
        let orthographic_direction = direction - (dot(direction, view_vec) * view_vec);
        let axis = normalize(cross(direction, view_vec)); // TODO: Why XeGTAO normalize? Paper does not
        let projected_normal = pixel_normal - axis * dot(pixel_normal, axis);
        let projected_normal_length = length(projected_normal);

        let sign_norm = sign(dot(orthographic_direction, projected_normal));
        let cos_norm = saturate(dot(projected_normal, view_vec) / projected_normal_length);
        let n = sign_norm * acos(cos_norm);

        for (var slice_side = 0u; slice_side < 2u; slice_side += 1u) {
            let side_modifier = -1.0 + (2.0 * f32(slice_side));
            let min_cos_horizon = n - (side_modifier * half_pi);
            var cos_horizon = min_cos_horizon;
            for (var s = 0u; s < samples_per_slice_side; s += 1u) {
                var sample_noise = (slice + f32(s) * f32(samples_per_slice_side)) * 0.6180339887498948482;
                sample_noise = fract(f32(noise.sample) + sample_noise);

                var sample = (f32(s) + sample_noise) / f32(samples_per_slice_side);
                sample = pow(sample, 2.1); // https://github.com/GameTechDev/XeGTAO#sample-distribution

                let sample_uv = uv + side_modifier * sample * vec2<f32>(omega.x, -omega.y);
                let sample_position = load_and_reconstruct_view_space_position(sample_uv);

                let sample_difference = sample_position - pixel_position;
                let sample_distance = length(sample_difference);
                let sample_horizon = sample_difference / sample_distance;

                let depth_range_scale_factor = 0.75;
                let effect_radius = depth_range_scale_factor * 0.5 * 1.457;
                let falloff_range = 0.615 * effect_radius;
                let falloff_from = effect_radius * (1.0 - 0.615);
                let falloff_mul = -1.0 / falloff_range;
                let falloff_add = falloff_from / falloff_range + 1.0;
                let weight = saturate(sample_distance * falloff_mul + falloff_add);
                var sample_cos_horizon = dot(sample_horizon, view_vec);
                sample_cos_horizon = mix(min_cos_horizon, sample_cos_horizon, weight);

                cos_horizon = max(cos_horizon, sample_cos_horizon);
            }

            let horizon = acos(cos_horizon);
            let horizon = n + clamp(side_modifier * horizon - n, -half_pi, half_pi);
            visiblity += projected_normal_length * (cos_norm + 2.0 * horizon * sin(n) - cos(2.0 * horizon - n)) / 4.0;
        }
    }
    visiblity /= f32(slice_count);

    textureStore(ambient_occlusion, pixel_coordinates, vec4<u32>(u32(visiblity), 0u, 0u, 0u));
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
