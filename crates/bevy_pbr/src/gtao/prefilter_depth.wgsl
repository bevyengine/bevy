// Inputs a depth texture (in screenspace) and outputs a MIP-chain of viewspace depths
// Because GTAO's performance is bound by texture reads, this increases performance over using the raw depth

@group(0) @binding(0) var input_depth: texture_2d<f32>;
@group(0) @binding(1) var prefiltered_depth_mip0: texture_storage_2d<r32float, write>;
@group(0) @binding(2) var prefiltered_depth_mip1: texture_storage_2d<r32float, write>;
@group(0) @binding(3) var prefiltered_depth_mip2: texture_storage_2d<r32float, write>;
@group(0) @binding(4) var prefiltered_depth_mip3: texture_storage_2d<r32float, write>;
@group(0) @binding(5) var prefiltered_depth_mip4: texture_storage_2d<r32float, write>;
@group(0) @binding(6) var point_clamp_sampler: sampler;

fn clamp_depth(depth: f32) -> f32 {
    return clamp(depth, 0.0, 3.402823466e+38);
}

fn screen_to_view_space_depth(depth: f32) -> f32 {
    // TODO
    return 0.0;
}

@compute
@workgroup_size(8, 8, 1)
fn prefilter_depth(@builtin(global_invocation_id) global_id: vec3<u32>, @builtin(local_invocation_id) local_id: vec3<u32>) {
    let base_coordinate = global_id.xy;
    var<workgroup> scratch_space: array<array<f32, 8>, 8>;

    // MIP 0
    let pixel_coordinate = vec2<i32>(base_coordinate) * 2i;
    let viewport_pixel_size = 1.0 / vec2<f32>(textureDimensions(input_depth));
    let depths4 = textureGather(0, input_depth, point_clamp_sampler, vec2<f32>(pixel_coordinate) * viewport_pixel_size, vec2<i32>(1i, 1i));
    let depth0 = clamp_depth(screen_to_view_space_depth(depths4.w));
    let depth1 = clamp_depth(screen_to_view_space_depth(depths4.z));
    let depth2 = clamp_depth(screen_to_view_space_depth(depths4.x));
    let depth3 = clamp_depth(screen_to_view_space_depth(depths4.y));
    textureStore(prefiltered_depth_mip0, pixel_coordinate, vec4<f32>(depth0, 0.0, 0.0, 0.0));
    textureStore(prefiltered_depth_mip0, pixel_coordinate + vec2<i32>(1i, 0i), vec4<f32>(depth0, 0.0, 0.0, 0.0));
    textureStore(prefiltered_depth_mip0, pixel_coordinate + vec2<i32>(0i, 1i), vec4<f32>(depth1, 0.0, 0.0, 0.0));
    textureStore(prefiltered_depth_mip0, pixel_coordinate + vec2<i32>(1i, 1i), vec4<f32>(depth2, 0.0, 0.0, 0.0));

    // MIP 1
    // TODO

    workgroupBarrier();

    // MIP 2
    // TODO

    workgroupBarrier();

    // MIP 3
    // TODO

    workgroupBarrier();

    // MIP 4
    // TODO
}
