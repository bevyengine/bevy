// Resolves a multisample depth buffer with the min operation.
//
// This is a workaround for multisample depth resolve not being available in
// `wgpu`.

#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var input_depth: texture_multisampled_2d<f32>;

@fragment
fn main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let coords = vec2<i32>(floor(in.uv * vec2<f32>(textureDimensions(input_depth))));

    // Take the minimum of every sample.
    var depth = textureLoad(input_depth, coords, 0).r;
    for (var sample = 1; sample < i32(textureNumSamples(input_depth)); sample += 1) {
        depth = min(depth, textureLoad(input_depth, coords, sample).r);
    }

    return vec4<f32>(depth, 0.0, 0.0, 0.0);
}
