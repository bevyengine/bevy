#import bevy_core_pipeline::fullscreen_vertex_shader

@group(0) @binding(0)
var texture: texture_2d<f32>;
@group(0) @binding(1)
var texture_sampler: sampler;

// https://gpuopen.com/learn/optimized-reversible-tonemapper-for-resolve/
fn tonemap(c: vec3<f32>) -> vec3<f32> { 
    return c * (1.0 / (max(c.r, max(c.g, c.b)) + 1.0)); 
}

fn tonemap_invert(c: vec3<f32>) -> vec3<f32> { 
    return c * (1.0 / (1.0 - max(c.r, max(c.g, c.b)))); 
}

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    let col = textureSample(texture, texture_sampler, in.uv);
    return vec4(tonemap(col.rgb), col.a);
}
