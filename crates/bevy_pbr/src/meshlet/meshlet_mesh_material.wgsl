#import bevy_pbr::{
    view_transformations::uv_to_ndc,
    meshlet_visibility_buffer_resolve::resolve_vertex_output,
}

fn rand_f(state: ptr<function, u32>) -> f32 {
    *state = *state * 747796405u + 2891336453u;
    let word = ((*state >> ((*state >> 28u) + 4u)) ^ *state) * 277803737u;
    return f32((word >> 22u) ^ word) * bitcast<f32>(0x2f800004u);
}

@vertex
fn vertex(@builtin(vertex_index) vertex_input: u32) -> @builtin(position) vec4<f32> {
    let vertex_index = vertex_input % 3u;
    let material_id = vertex_input / 3u;
    let material_depth = f32(material_id) / 65535.0;
    let uv = vec2<f32>(vec2(vertex_index >> 1u, vertex_index & 1u)) * 2.0;
    return vec4(uv_to_ndc(uv), material_depth, 1.0);
}

@fragment
fn fragment(@builtin(position) frag_coord: vec4<f32>) -> @location(0) vec4<f32> {
    let vertex_output = resolve_vertex_output(frag_coord);

    var rng = vertex_output.meshlet_id;
    let color = vec3(rand_f(&rng), rand_f(&rng), rand_f(&rng));
    return vec4(color, 1.0);
}
