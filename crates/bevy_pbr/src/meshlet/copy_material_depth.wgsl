#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var material_depth: texture_2d<u32>;

/// This pass copies the R16Uint material depth texture to an actual Depth16Unorm depth texture.

@fragment
fn copy_material_depth(in: FullscreenVertexOutput) -> @builtin(frag_depth) f32 {
    return f32(textureLoad(material_depth, vec2<i32>(in.position.xy), 0).r) / 65535.0;
}
