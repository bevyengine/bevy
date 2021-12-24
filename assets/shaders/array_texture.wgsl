#import bevy_pbr::mesh_view_types
#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_types
#import bevy_pbr::mesh_bindings
// NOTE: Bindings must come before functions that use them!
#import bevy_pbr::mesh_functions

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
    [[location(0)]] world_position: vec4<f32>;
};

[[stage(vertex)]]
fn vertex(vertex: Vertex) -> VertexOutput {
    let world_position = mesh_model_position_to_world(vec4<f32>(vertex.position, 1.0));
    var out: VertexOutput;
    out.clip_position = mesh_world_position_to_clip(world_position);
    out.world_position = world_position;
    return out;
}

struct FragmentInput {
    [[location(0)]] world_position: vec4<f32>;
};

[[stage(fragment)]]
fn fragment(in: FragmentInput) -> [[location(0)]] vec4<f32> {
    // Screen-space coordinates determine which layer of the array texture we sample.
    let ss = in.world_position.xy / in.world_position.w;
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
