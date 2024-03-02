#define_import_path bevy_ui::ui_vertex_output

// The Vertex output of the default vertex shader for the Ui Material pipeline.
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    // The size of the borders in UV space. Order is Left, Right, Top, Bottom.
    @location(1) border_widths: vec4<f32>,
    // The size of the node in pixels. Order is width, height.
    @location(2) @interpolate(flat) size: vec2<f32>,
    @builtin(position) position: vec4<f32>,
};
