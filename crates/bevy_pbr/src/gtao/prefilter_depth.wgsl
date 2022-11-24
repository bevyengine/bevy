// Inputs a depth texture (in screenspace) and outputs a MIP-chain of viewspace depths
// Because GTAO's performance is bound by texture reads, this increases performance over using the raw depth

#import bevy_pbr::mesh_view_types

@group(0) @binding(0) var input_depth: texture_2d<f32>;
@group(0) @binding(1) var prefiltered_depth_mip0: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var prefiltered_depth_mip1: texture_storage_2d<r32float, write>;
@group(0) @binding(3) var prefiltered_depth_mip2: texture_storage_2d<r32float, write>;
@group(0) @binding(4) var prefiltered_depth_mip3: texture_storage_2d<r32float, write>;
@group(0) @binding(5) var prefiltered_depth_mip4: texture_storage_2d<r32float, write>;
@group(0) @binding(6) var point_clamp_sampler: sampler;
@group(0) @binding(7) var<uniform> view: View;

fn clamp_depth(depth: f32) -> f32 {
    return clamp(depth, 0.0, 3.402823466e+38);
}

fn screen_to_view_space_depth(depth: f32) -> f32 {
    // float depthLinearizeMul = (rowMajor)?(-projMatrix[3 * 4 + 2]):(-projMatrix[3 + 2 * 4]);     // float depthLinearizeMul = ( clipFar * clipNear ) / ( clipFar - clipNear );
    // float depthLinearizeAdd = (rowMajor)?( projMatrix[2 * 4 + 2]):( projMatrix[2 + 2 * 4]);     // float depthLinearizeAdd = clipFar / ( clipFar - clipNear );

    // // correct the handedness issue. need to make sure this below is correct, but I think it is.
    // if( depthLinearizeMul * depthLinearizeAdd < 0 )
    //     depthLinearizeAdd = -depthLinearizeAdd;

    // // Optimised version of "-cameraClipNear / (cameraClipFar - projDepth * (cameraClipFar - cameraClipNear)) * cameraClipFar"
    // return depthLinearizeMul / (depthLinearizeAdd - depth);

    // TODO
    return 0.0;
}

fn depth_for_next_mip(depth0: f32, depth1: f32, depth2: f32, depth3: f32) -> f32 {
    // TODO
    return 0.0;
}

// Used to share the depths from the previous MIP level with all invocations per workgroup
var<workgroup> workspace: array<array<f32, 8>, 8>;

@compute
@workgroup_size(8, 8, 1)
fn prefilter_depth(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let base_coordinates = vec2<i32>(global_id.xy);

    // MIP 0 - Copy 4 texels from the input depth (per invocation, 8x8 invocations)
    let pixel_coordinates = base_coordinates * 2i;
    let viewport_pixel_size = 1.0 / view.viewport.zw;
    let depths4 = textureGather(0, input_depth, point_clamp_sampler, vec2<f32>(pixel_coordinates) * viewport_pixel_size, vec2<i32>(1i, 1i));
    let depth0 = clamp_depth(screen_to_view_space_depth(depths4.w));
    let depth1 = clamp_depth(screen_to_view_space_depth(depths4.z));
    let depth2 = clamp_depth(screen_to_view_space_depth(depths4.x));
    let depth3 = clamp_depth(screen_to_view_space_depth(depths4.y));
    textureStore(prefiltered_depth_mip0, pixel_coordinates + vec2<i32>(0i, 0i), vec4<f32>(depth0, 0.0, 0.0, 0.0));
    textureStore(prefiltered_depth_mip0, pixel_coordinates + vec2<i32>(1i, 0i), vec4<f32>(depth0, 0.0, 0.0, 0.0));
    textureStore(prefiltered_depth_mip0, pixel_coordinates + vec2<i32>(0i, 1i), vec4<f32>(depth1, 0.0, 0.0, 0.0));
    textureStore(prefiltered_depth_mip0, pixel_coordinates + vec2<i32>(1i, 1i), vec4<f32>(depth2, 0.0, 0.0, 0.0));

    // MIP 1 - Weighted average of MIP 0's depth values (per invocation, 8x8 invocations)
    let depth_mip1 = depth_for_next_mip(depth0, depth1, depth2, depth3);
    textureStore(prefiltered_depth_mip1, base_coordinates, vec4<f32>(depth_mip1, 0.0, 0.0, 0.0));
    workspace[local_id.x][local_id.y] = depth_mip1;

    workgroupBarrier();

    // MIP 2 - Weighted average of MIP 1's depth values (per invocation, 4x4 invocations)
    if all(local_id.xy % vec2<u32>(2u) == vec2<u32>(0u)) {
        let depth0 = workspace[local_id.x + 0u][local_id.y + 0u];
        let depth1 = workspace[local_id.x + 1u][local_id.y + 0u];
        let depth2 = workspace[local_id.x + 0u][local_id.y + 1u];
        let depth3 = workspace[local_id.x + 1u][local_id.y + 1u];
        let depth_mip2 = depth_for_next_mip(depth0, depth1, depth2, depth3);
        textureStore(prefiltered_depth_mip2, base_coordinates / 2i, vec4<f32>(depth_mip2, 0.0, 0.0, 0.0));
        workspace[local_id.x][local_id.y] = depth_mip2;
    }

    workgroupBarrier();

    // MIP 3 - Weighted average of MIP 2's depth values (per invocation, 2x2 invocations)
    if all(local_id.xy % vec2<u32>(4u) == vec2<u32>(0u)) {
        let depth0 = workspace[local_id.x + 0u][local_id.y + 0u];
        let depth1 = workspace[local_id.x + 2u][local_id.y + 0u];
        let depth2 = workspace[local_id.x + 0u][local_id.y + 2u];
        let depth3 = workspace[local_id.x + 2u][local_id.y + 2u];
        let depth_mip3 = depth_for_next_mip(depth0, depth1, depth2, depth3);
        textureStore(prefiltered_depth_mip3, base_coordinates / 2i, vec4<f32>(depth_mip3, 0.0, 0.0, 0.0));
        workspace[local_id.x][local_id.y] = depth_mip3;
    }

    workgroupBarrier();

    // MIP 4 - Weighted average of MIP 3's depth values (per invocation, 1 invocation)
    if all(local_id.xy % vec2<u32>(8u) == vec2<u32>(0u)) {
        let depth0 = workspace[local_id.x + 0u][local_id.y + 0u];
        let depth1 = workspace[local_id.x + 4u][local_id.y + 0u];
        let depth2 = workspace[local_id.x + 0u][local_id.y + 4u];
        let depth3 = workspace[local_id.x + 4u][local_id.y + 4u];
        let depth_mip4 = depth_for_next_mip(depth0, depth1, depth2, depth3);
        textureStore(prefiltered_depth_mip4, base_coordinates / 2i, vec4<f32>(depth_mip4, 0.0, 0.0, 0.0));
    }
}
