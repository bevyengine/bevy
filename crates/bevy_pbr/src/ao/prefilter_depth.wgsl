// Inputs a depth texture (in screenspace) and outputs a MIP-chain of viewspace depths
// Because GTAO's performance is bound by texture reads, this increases performance over using the raw depth

#import bevy_pbr::mesh_view_types

struct AmbientOcclusionSettings {
    effect_radius: f32,
    effect_falloff_range: f32,
};

@group(0) @binding(0) var input_depth: texture_2d<f32>;
@group(0) @binding(1) var prefiltered_depth_mip0: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var prefiltered_depth_mip1: texture_storage_2d<r32float, write>;
@group(0) @binding(3) var prefiltered_depth_mip2: texture_storage_2d<r32float, write>;
@group(0) @binding(4) var prefiltered_depth_mip3: texture_storage_2d<r32float, write>;
@group(0) @binding(5) var prefiltered_depth_mip4: texture_storage_2d<r32float, write>;
@group(0) @binding(6) var point_clamp_sampler: sampler;
@group(0) @binding(7) var<uniform> ao_settings: AmbientOcclusionSettings;
@group(0) @binding(8) var<uniform> view: View;

fn linearize_depth(depth: f32) -> f32 {
    return view.projection[3][2] / depth;
}

// Using 4 depths from the previous MIP, compute a weighted average for the depth of the current MIP
fn weighted_average(depth0: f32, depth1: f32, depth2: f32, depth3: f32) -> f32 {
    // TODO: Cleanup constants
    let effect_radius = 0.75 * ao_settings.effect_radius * 1.457;
    let falloff_range = 0.615 * effect_radius;
    let falloff_from = effect_radius * (1.0 - ao_settings.effect_falloff_range);
    let falloff_mul = -1.0 / falloff_range;
    let falloff_add = falloff_from / falloff_range + 1.0;
    let max_depth = max(max(depth0, depth1), max(depth2, depth3));

    let weight0 = saturate((max_depth - depth0) * falloff_mul + falloff_add);
    let weight1 = saturate((max_depth - depth1) * falloff_mul + falloff_add);
    let weight2 = saturate((max_depth - depth2) * falloff_mul + falloff_add);
    let weight3 = saturate((max_depth - depth3) * falloff_mul + falloff_add);
    let weight_total = weight0 + weight1 + weight2 + weight3;

    return ((weight0 * depth0) + (weight1 * depth1) + (weight2 * depth2) + (weight3 * depth3)) / weight_total;
}

// Used to share the depths from the previous MIP level between all invocations in a workgroup
var<workgroup> previous_mip_depth: array<array<f32, 8>, 8>;

@compute
@workgroup_size(8, 8, 1)
fn prefilter_depth(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let base_coordinates = vec2<i32>(global_id.xy);

    // MIP 0 - Copy 4 texels from the input depth (per invocation, 8x8 invocations)
    let pixel_coordinates = base_coordinates * 2i;
    let viewport_pixel_size = 1.0 / view.viewport.zw;
    let depths4 = textureGather(0, input_depth, point_clamp_sampler, vec2<f32>(pixel_coordinates) * viewport_pixel_size, vec2<i32>(1i, 1i));
    let depth0 = linearize_depth(depths4.w);
    let depth1 = linearize_depth(depths4.z);
    let depth2 = linearize_depth(depths4.x);
    let depth3 = linearize_depth(depths4.y);
    textureStore(prefiltered_depth_mip0, pixel_coordinates + vec2<i32>(0i, 0i), vec4<f32>(depth0, 0.0, 0.0, 0.0));
    textureStore(prefiltered_depth_mip0, pixel_coordinates + vec2<i32>(1i, 0i), vec4<f32>(depth0, 0.0, 0.0, 0.0));
    textureStore(prefiltered_depth_mip0, pixel_coordinates + vec2<i32>(0i, 1i), vec4<f32>(depth1, 0.0, 0.0, 0.0));
    textureStore(prefiltered_depth_mip0, pixel_coordinates + vec2<i32>(1i, 1i), vec4<f32>(depth2, 0.0, 0.0, 0.0));

    // MIP 1 - Weighted average of MIP 0's depth values (per invocation, 8x8 invocations)
    let depth_mip1 = weighted_average(depth0, depth1, depth2, depth3);
    textureStore(prefiltered_depth_mip1, base_coordinates, vec4<f32>(depth_mip1, 0.0, 0.0, 0.0));
    previous_mip_depth[local_id.x][local_id.y] = depth_mip1;

    workgroupBarrier();

    // MIP 2 - Weighted average of MIP 1's depth values (per invocation, 4x4 invocations)
    if all(local_id.xy % vec2<u32>(2u) == vec2<u32>(0u)) {
        let depth0 = previous_mip_depth[local_id.x + 0u][local_id.y + 0u];
        let depth1 = previous_mip_depth[local_id.x + 1u][local_id.y + 0u];
        let depth2 = previous_mip_depth[local_id.x + 0u][local_id.y + 1u];
        let depth3 = previous_mip_depth[local_id.x + 1u][local_id.y + 1u];
        let depth_mip2 = weighted_average(depth0, depth1, depth2, depth3);
        textureStore(prefiltered_depth_mip2, base_coordinates / 2i, vec4<f32>(depth_mip2, 0.0, 0.0, 0.0));
        previous_mip_depth[local_id.x][local_id.y] = depth_mip2;
    }

    workgroupBarrier();

    // MIP 3 - Weighted average of MIP 2's depth values (per invocation, 2x2 invocations)
    if all(local_id.xy % vec2<u32>(4u) == vec2<u32>(0u)) {
        let depth0 = previous_mip_depth[local_id.x + 0u][local_id.y + 0u];
        let depth1 = previous_mip_depth[local_id.x + 2u][local_id.y + 0u];
        let depth2 = previous_mip_depth[local_id.x + 0u][local_id.y + 2u];
        let depth3 = previous_mip_depth[local_id.x + 2u][local_id.y + 2u];
        let depth_mip3 = weighted_average(depth0, depth1, depth2, depth3);
        textureStore(prefiltered_depth_mip3, base_coordinates / 4i, vec4<f32>(depth_mip3, 0.0, 0.0, 0.0));
        previous_mip_depth[local_id.x][local_id.y] = depth_mip3;
    }

    workgroupBarrier();

    // MIP 4 - Weighted average of MIP 3's depth values (per invocation, 1 invocation)
    if all(local_id.xy % vec2<u32>(8u) == vec2<u32>(0u)) {
        let depth0 = previous_mip_depth[local_id.x + 0u][local_id.y + 0u];
        let depth1 = previous_mip_depth[local_id.x + 4u][local_id.y + 0u];
        let depth2 = previous_mip_depth[local_id.x + 0u][local_id.y + 4u];
        let depth3 = previous_mip_depth[local_id.x + 4u][local_id.y + 4u];
        let depth_mip4 = weighted_average(depth0, depth1, depth2, depth3);
        textureStore(prefiltered_depth_mip4, base_coordinates / 8i, vec4<f32>(depth_mip4, 0.0, 0.0, 0.0));
    }
}
