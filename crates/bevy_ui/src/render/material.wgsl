#import bevy_render::view  View
#define_import_path bevy_ui::ui_vertex_output

@group(0) @binding(0)
var<uniform> view: View;

struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    @location(1) color: vec4<f32>,
    @builtin(position) position: vec4<f32>,
};

@vertex
fn vertex(
    @location(0) vertex_position: vec3<f32>,
    @location(1) vertex_uv: vec2<f32>,
    @location(2) vertex_color: vec4<f32>,
) -> UiVertexOutput {
    var out: UiVertexOutput;
    out.uv = vertex_uv;
    out.position = view.view_proj * vec4<f32>(vertex_position, 1.0);
    out.color = vertex_color;
    return out;
}

@fragment
fn fragment(in: UiVertexOutput) -> @location(0) vec4<f32> {
    return vec4<f32>(1.0);
}
