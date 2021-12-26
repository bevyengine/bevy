#import bevy_pbr::mesh_view_types
#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_functions
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_bindings

[[group(1), binding(0)]]
var my_array_texture: texture_2d_array<f32>;
[[group(1), binding(1)]]
var my_array_texture_sampler: sampler;

struct Vertex {
    [[location(0)]] position: vec3<f32>;
    [[location(1)]] normal: vec3<f32>;
    [[location(2)]] uv: vec2<f32>;
};

struct VertexOutput {
    [[builtin(position)]] clip_position: vec4<f32>;
    [[location(0)]] position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = mesh_model_position_to_clip(
        mesh.model,
        view.view_proj,
        vec4<f32>(vertex.position, 1.0)
    );
    out.position = out.clip_position;
    return out;
}

struct FragmentInput {
    [[location(0)]] clip_position: vec4<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    // Screen-space coordinates determine which layer of the array texture we sample.
    let ss = in.clip_position.xy / in.clip_position.w;
    var layer: f32 = 0.0;
    if (ss.x > 0.0 && ss.y > 0.0) {
        layer = 0.0;
    } else if (ss.x < 0.0 && ss.y > 0.0) {
        layer = 1.0;
    } else if (ss.x > 0.0 && ss.y < 0.0) {
        layer = 2.0;
    } else {
        layer = 3.0;
    }

    // Convert to texture coordinates.
    let uv = (ss + vec2<f32>(1.0)) / 2.0;

    return textureSampleLevel(my_array_texture, my_array_texture_sampler, uv, i32(layer), 0.0);
}
