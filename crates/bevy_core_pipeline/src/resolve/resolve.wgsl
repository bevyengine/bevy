#import bevy_core_pipeline::fullscreen_vertex_shader::FullscreenVertexOutput

@group(0) @binding(0) var in_texture: texture_multisampled_2d<f32>;

const SAMPLE_COUNT: u32 = #{SAMPLE_COUNT};

@fragment
fn fs_main(in: FullscreenVertexOutput) -> @location(0) vec4<f32> {
    var out: vec4<f32> = vec4<f32>(0.0);
    for (var i = 0u; i < SAMPLE_COUNT; i++){
        out += textureLoad(in_texture, vec2<i32>(in.position.xy), i);
    }
    out /= f32(SAMPLE_COUNT);
    return out;
}
