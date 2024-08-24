#import bevy_render::view::View;
#import bevy_render::globals::Globals;

@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var<uniform> globals: Globals;
@group(0) @binding(2)
var<uniform> slice_geom: SliceGeom;

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct SliceGeom {
    left: f32,
    right: f32,
    top: f32,
    bottom: f32,
}

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
}

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(1) vertex_color: vec2<f32>,
) -> VertexOutput {
    var out: UiVertexOutput;
    out.uv = vertex_uv;
    out.position = view.clip_from_world * vec4<f32>(vertex_position, 1.0);
    return out;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    return textureSample(sprite_sampler, sprite_texture, in.uv);
}
