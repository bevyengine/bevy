import super::{
    bindings::{globals, view, VertexOutput},
};
enable wgpu_mesh_shader;

// TODO: import these functions in the future when doing so does not crash the shader.
// Convert ndc depth to linear view z.
// Note: Depth values in front of the camera will be negative as -z is forward
fn depth_ndc_to_view_z(ndc_depth: f32, clip_from_view: mat4x4<f32>, view_from_clip: mat4x4<f32>) -> f32 {
// #ifdef VIEW_PROJECTION_PERSPECTIVE
    return -perspective_camera_near(clip_from_view) / ndc_depth;
// #else ifdef VIEW_PROJECTION_ORTHOGRAPHIC
//     return -(clip_from_view[3][2] - ndc_depth) / clip_from_view[2][2];
// #else
//     let view_pos = view_from_clip * vec4(0.0, 0.0, ndc_depth, 1.0);
//     return view_pos.z / view_pos.w;
// #endif
}

fn perspective_camera_near(clip_from_view: mat4x4<f32>) -> f32 {
    return clip_from_view[3][2];
}

struct PrimitiveInput {
    @per_primitive @location(3) colorMask: vec4<f32>,
}

struct Output {
    @location(0) color: vec4<f32>,
    @builtin(frag_depth) frag_depth: f32
}

@fragment
fn fragment(vertex: VertexOutput, primitive: PrimitiveInput) -> Output {
    var output: Output;
    // output.color = vec4(vertex.uv, 0.,1.);
    // output.color = vec4(vertex.normal,1.);
    output.color = primitive.colorMask;
    output.frag_depth = vertex.position.z;
    return output;
}