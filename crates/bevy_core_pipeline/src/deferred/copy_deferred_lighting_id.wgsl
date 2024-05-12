#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0)
var material_id_texture: texture_2d<u32>;

struct FragmentOutput {
    @builtin(frag_depth) frag_depth: f32,

}

@fragment
fn fragment(in: FullscreenVertexOutput) -> FragmentOutput {
    var out: FragmentOutput;
    // Depth is stored as unorm, so we are dividing the u8 by 255.0 here.
    out.frag_depth = f32(textureLoad(material_id_texture, vec2<i32>(in.position.xy), 0).x) / 255.0;
    return out;
}

