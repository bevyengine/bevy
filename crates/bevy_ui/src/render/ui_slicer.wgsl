#import bevy_render::view::View;
#import bevy_render::globals::Globals;

@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var<uniform> globals: Globals;

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

// @group(2) @binding(1)
// var<uniform> slice_geom: SliceGeom;


// struct SliceGeom {
//     left: f32,
//     right: f32,
//     top: f32,
//     bottom: f32,
// }

struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
) -> UiVertexOutput {
    var out: UiVertexOutput;
    out.uv = vertex_uv;
    out.color = vertex_color;
    out.position = view.clip_from_world * vec4<f32>(vertex_position, 1.0);
    return out;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    return in.color * textureSample(sprite_texture, sprite_sampler, in.uv);
}
