#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var input_depth: texture_2d<f32>;
@group(0) @binding(1) var samplr: sampler;

/// Performs a 2x2 downsample on a depth texture to generate the next mip level of a hierarchical depth buffer.

@fragment
fn downsample_depth(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let depth_quad = textureGather(0, input_depth, samplr, in.uv);
    let downsampled_depth = min(
        min(depth_quad.x, depth_quad.y),
        min(depth_quad.z, depth_quad.w),
    );
    return vec4(downsampled_depth, 0.0, 0.0, 0.0);
}
