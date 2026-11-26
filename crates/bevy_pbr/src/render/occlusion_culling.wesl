//! Occlusion culling utility functions.

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
    let max_texel = depth_pyramid_size - 1.0;
    let aabb_top_left = aabb.xy * depth_pyramid_size;
    let texel0 = vec2<u32>(clamp(aabb_top_left, vec2(0.0), max_texel));
    let texel1 = vec2<u32>(clamp(aabb_top_left + 1.0, vec2(0.0), max_texel));

    let depth_quad_a = textureLoad(depth_pyramid, texel0, depth_level).x;
    let depth_quad_b = textureLoad(depth_pyramid, vec2(texel1.x, texel0.y), depth_level).x;
    let depth_quad_c = textureLoad(depth_pyramid, vec2(texel0.x, texel1.y), depth_level).x;
    let depth_quad_d = textureLoad(depth_pyramid, texel1, depth_level).x;
    return min(min(depth_quad_a, depth_quad_b), min(depth_quad_c, depth_quad_d));
}
