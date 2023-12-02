#define_import_path bevy_sprite::sprite_vertex_output

// The Vertex output of the default vertex shader for the Sprite Material pipeline.
struct SpriteVertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) color: vec4<f32>,
};
