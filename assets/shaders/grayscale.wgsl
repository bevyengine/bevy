#import bevy_sprite::sprite_vertex_output::SpriteVertexOutput

// Bind the sprite texture and sampler to the first binding in the first group
@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

// Fragment shader entry point
@fragment
fn fragment(in: SpriteVertexOutput) -> @location(0) vec4<f32> {
    // Calculate the color of the fragment by multiplying the input color with the sampled texture color
    var color = in.color * textureSample(sprite_texture, sprite_sampler, in.uv);

    // Convert the color to grayscale using the formula:
    // gray = 0.21 * red + 0.72 * green + 0.07 * blue
    let g = 0.21 * color.r + 0.72 * color.g + 0.07 * color.b;

    // Return the grayscale color with the same alpha value as the input color
    return vec4<f32>(g, g, g, color.a);
}