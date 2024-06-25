// `custom_phase_item.wgsl`
//
// This shader goes with the `custom_phase_item` example. It demonstrates how to
// enqueue custom rendering logic in a `RenderPhase`.

// The GPU-side vertex structure.
struct Vertex {
    // The world-space position of the vertex.
    @location(0) position: vec3<f32>,
    // The color of the vertex.
    @location(1) color: vec3<f32>,
};

// Information passed from the vertex shader to the fragment shader. (The name
// comes from OpenGL.)
struct Varyings {
    // The clip-space position of the vertex.
    @builtin(position) clip_position: vec4<f32>,
    // The color of the vertex.
    @location(0) color: vec3<f32>,
};

// The vertex shader entry point.
@vertex
fn vertex(vertex: Vertex) -> Varyings {
    // Use an orthographic projection.
    var varyings: Varyings;
    varyings.clip_position = vec4(vertex.position.xyz, 1.0);
    varyings.color = vertex.color;
    return varyings;
}

// The fragment shader entry point.
@fragment
fn fragment(varyings: Varyings) -> @location(0) vec4<f32> {
    return vec4(varyings.color, 1.0);
}
