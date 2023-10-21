#define_import_path bevy_ui::ui_vertex_output

// The Vertex output of the default fragment shader for the Ui Material pipeline.
struct UiVertexOutput {
    @location(0) uv: vec2<f32>,
    // The size of the borders in UV space. order is Left, Right, Top, Bottom.
    @location(1) border_widths: vec4<f32>,
    @builtin(position) position: vec4<f32>,
};
