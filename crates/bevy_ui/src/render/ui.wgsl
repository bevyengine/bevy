#import bevy_render::view

const TEXTURED_QUAD: u32 = 0u;
const INVERT_CORNERS: u32 = 2u;

@group(0) @binding(0)
var<uniform> view: View;

struct VertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @location(2) @interpolate(flat) size: vec2<f32>,
    @location(3) @interpolate(flat) mode: u32,
    @location(4) @interpolate(flat) radius: f32,    
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
    @location(3) mode: u32,
    @location(4) radius: f32,
    @location(5) size: vec2<f32>,
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = vertex_color;
    out.mode = mode;
    out.radius = radius;
    out.size = size;
    return out;
}

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
@group(1) @binding(1)
var sprite_sampler: sampler;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // textureSample can only be called in unform control flow, not inside an if branch.
    var color = textureSample(sprite_texture, sprite_sampler, in.uv);

    if in.mode == TEXTURED_QUAD {
        color = in.color * color;
    } else {
        color = in.color;
    }

    if in.radius <= 0. {
        return color;
    }

    let point = abs(in.uv - vec2<f32>(0.5, 0.5)) * in.size;
    let inner = 0.5 * in.size - vec2<f32>(in.radius, in.radius);

    if in.mode == INVERT_CORNERS {
        if inner.x < point.x && inner.y < point.y {
            let c = point - inner;
            let distance = in.radius - length(c);
            if  dot(c, c) <= in.radius * in.radius  {
                color[3] = 0.0;
            }
        } else {
            color[3] = 0.0;
        }
    } else {
        if inner.x < point.x && inner.y < point.y {
            let c = point - inner;
            let distance = in.radius - length(c);
            if in.radius * in.radius < dot(c, c) {
                color[3] = 0.0;
            }
        }
    } 
    return color;
}
