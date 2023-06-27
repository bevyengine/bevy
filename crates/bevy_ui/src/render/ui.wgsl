#import bevy_render::view

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
    
    let q = (abs(in.uv - vec2<f32>(0.5, 0.5)) - 0.5) * in.size + in.radius;
    let d = min(max(q.x, q.y), 0.0) + length(max(q, vec2<f32>(0.0))) - in.radius;

    switch in.mode {
        // Textured rect
        case 0u: {
            return mix(in.color * color, vec4<f32>(0.0), smoothstep(-1.0, 0.5, d));
        }
        // Untextured rect
        case 1u: {
            return mix(in.color, vec4<f32>(0.0), smoothstep(-1.0, 0.5, d));
        }
        // Inverted rect (fills outside the rounded corners)
        case 2u, default: {
            return mix(vec4<f32>(0.0), in.color, smoothstep(-1.0, 1.0, d));
        }
    }
}

