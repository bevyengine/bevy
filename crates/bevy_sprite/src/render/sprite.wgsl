#ifdef TONEMAP_IN_SHADER
#import bevy_core_pipeline::tonemapping
#endif

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

@group(1) @binding(0)
var sprite_texture: texture_2d<f32>;
@group(1) @binding(1)
var sprite_sampler: sampler;

@group(2) @binding(0)
var<storage, read> entity_indices: array<u32>;

@group(3) @binding(0)
var<uniform> entity_indices_offset: u32;

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
#ifdef COLORED
    @location(1) color: vec4<f32>,
#endif
    @location(2) vertex_index: u32,
};

@vertex
fn vertex(
    @builtin(vertex_index) vertex_index: u32,
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
#ifdef COLORED
    @location(2) vertex_color: vec4<f32>,
#endif
) -> VertexOutput {
    var out: VertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.vertex_index = u32((vertex_index - entity_indices_offset) / 6u);
#ifdef COLORED
    out.color = vertex_color;
#endif
    return out;
}

struct FragmentOutput {
    @location(0) color: vec4<f32>,
#ifdef PICKING
    @location(1) picking: vec4<f32>,
#endif
}

@fragment
fn fragment(in: VertexOutput) -> FragmentOutput {
    var color = textureSample(sprite_texture, sprite_sampler, in.uv);
#ifdef COLORED
    color = in.color * color;
#endif

#ifdef TONEMAP_IN_SHADER
    color = vec4<f32>(reinhard_luminance(color.rgb), color.a);
#endif

    var out: FragmentOutput;
    out.color = color;

#ifdef PICKING
    out.picking = vec4(entity_index_to_vec3_f32(entity_indices[in.vertex_index]), picking_alpha(color.a));
#endif

    return out;
}
