#import bevy_pbr::forward_io::VertexOutput

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var depth_texture: texture_depth_2d;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var depth_sampler: sampler_comparison;

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    // Just draw the depth.
    let st = vec2<i32>(input.uv * vec2<f32>(textureDimensions(depth_texture).xy));
    return vec4(vec3(textureLoad(depth_texture, st, 0)), 1.0);
}