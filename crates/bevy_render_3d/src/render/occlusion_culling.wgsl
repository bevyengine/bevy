// Occlusion culling utility functions.

#define_import_path bevy_pbr::occlusion_culling

fn get_aabb_size_in_pixels(aabb: vec4<f32>, depth_pyramid: texture_2d<f32>) -> vec2<f32> {
    let depth_pyramid_size_mip_0 = vec2<f32>(textureDimensions(depth_pyramid, 0));
    let aabb_width_pixels = (aabb.z - aabb.x) * depth_pyramid_size_mip_0.x;
    let aabb_height_pixels = (aabb.w - aabb.y) * depth_pyramid_size_mip_0.y;
    return vec2(aabb_width_pixels, aabb_height_pixels);
}

fn get_occluder_depth(
    aabb: vec4<f32>,
    aabb_pixel_size: vec2<f32>,
    depth_pyramid: texture_2d<f32>
) -> f32 {
    let aabb_width_pixels = aabb_pixel_size.x;
    let aabb_height_pixels = aabb_pixel_size.y;

    let depth_pyramid_size_mip_0 = vec2<f32>(textureDimensions(depth_pyramid, 0));
    let depth_level = max(0, i32(ceil(log2(max(aabb_width_pixels, aabb_height_pixels))))); // TODO: Naga doesn't like this being a u32
    let depth_pyramid_size = vec2<f32>(textureDimensions(depth_pyramid, depth_level));
    let aabb_top_left = vec2<u32>(aabb.xy * depth_pyramid_size);

    let depth_quad_a = textureLoad(depth_pyramid, aabb_top_left, depth_level).x;
    let depth_quad_b = textureLoad(depth_pyramid, aabb_top_left + vec2(1u, 0u), depth_level).x;
    let depth_quad_c = textureLoad(depth_pyramid, aabb_top_left + vec2(0u, 1u), depth_level).x;
    let depth_quad_d = textureLoad(depth_pyramid, aabb_top_left + vec2(1u, 1u), depth_level).x;
    return min(min(depth_quad_a, depth_quad_b), min(depth_quad_c, depth_quad_d));
}
