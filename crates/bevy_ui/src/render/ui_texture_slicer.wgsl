#import bevy_render::view::View;
#import bevy_render::globals::Globals;

@group(0) @binding(0)
var<uniform> view: View;
@group(0) @binding(1)
var<uniform> globals: Globals;

@group(1) @binding(0) var sprite_texture: texture_2d<f32>;
@group(1) @binding(1) var sprite_sampler: sampler;

struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) slices: vec4<f32>,
    @location(3) @interpolate(flat) border: vec4<f32>,
    @location(4) @interpolate(flat) repeat: vec4<f32>,
    @builtin(position) position: vec4<f32>,
}

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) slices: vec4<f32>,
    @location(4) border: vec4<f32>,
    @location(5) repeat: vec4<f32>,
) -> UiVertexOutput {
    var out: UiVertexOutput;
    out.uv = vertex_uv;
    out.color = vertex_color;
    out.position = view.clip_from_world * vec4<f32>(vertex_position, 1.0);
    out.slices = slices;
    out.border = border;
    out.repeat = repeat;
    return out;
}

fn map_repeat(
    p: f32,
    r: f32
) -> f32 {
    return fract(p * r);
}

fn map_axis(
    p: f32,
    tl: f32,
    th: f32,
    il: f32,
    ih: f32,
) -> f32 {
    if p < il {
        return (p / il) * tl;
    } else if ih < p {
        return th + ((p - ih) / (1 - ih)) * (1 - th);
    } else {
        return tl + ((p - il) / (ih - il)) * (th - tl);
    }
}

fn map_uvs(
    uv: vec2<f32>,
    slices: vec4<f32>,
    border: vec4<f32>,
) -> vec2<f32> {
    let x = map_axis(uv.x, slices.x, slices.z, border.x, border.z);
    let y = map_axis(uv.y, slices.y, slices.w, border.y, border.w);
    return vec2(x, y);
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    let uv = map_uvs(in.uv, in.slices, in.border);
    return in.color * textureSample(sprite_texture, sprite_sampler, uv);
}
