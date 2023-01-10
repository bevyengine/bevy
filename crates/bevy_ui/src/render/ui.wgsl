#ifdef PICKING
#import bevy_core_pipeline::picking
#endif

struct View {
    view_proj: mat4x4<f32>,
    inverse_view_proj: mat4x4<f32>,
    view: mat4x4<f32>,
    inverse_view: mat4x4<f32>,
    projection: mat4x4<f32>,
    inverse_projection: mat4x4<f32>,
    world_position: vec3<f32>,
    // viewport(x_origin, y_origin, width, height)
    viewport: vec4<f32>,
};
@group(0) @binding(0)
var<uniform> view: View;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
#ifdef PICKING
    @location(2) entity_index: u32,
#endif
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
#ifdef PICKING
    @location(3) entity_index: u32,
#endif
) -> VertexOutput {
    var out: VertexOutput;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.uv = vertex_uv;
    out.color = vertex_color;
#ifdef PICKING
    out.entity_index = entity_index;
#endif
    return out;
}

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
@group(1) @binding(1)
var sprite_sampler: sampler;

struct FragmentOutput {
    @location(0) color: vec4<f32>,
#ifdef PICKING
    @location(1) picking: vec4<f32>,
#endif
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var color = textureSample(sprite_texture, sprite_sampler, in.uv);
    color = in.color * color;

    var out: FragmentOutput;

    out.color = color;

#ifdef PICKING
    out.picking = vec4(entity_index_to_vec3_f32(in.entity_index), picking_alpha(color.a));
#endif

    return out;
}
