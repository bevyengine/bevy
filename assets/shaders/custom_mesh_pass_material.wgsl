#import bevy_pbr::{
    pbr_bindings,
    pbr_types,
    mesh_functions,
    mesh_view_bindings,
    view_transformations,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var<uniform> outline_color: vec4<f32>;

const OUTLINE_WIDTH = 0.1;

struct Vertex {
    @builtin(instance_index) instance_index: u32,
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
};

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_position: vec4<f32>,
    @location(1) world_normal: vec3<f32>,
};

@vertex
fn vertex(vertex: Vertex) -> VertexOutput {
    var out: VertexOutput;

    // This only works when the mesh is at the origin.
    let expanded_position = vertex.position * (1 + OUTLINE_WIDTH);

    var world_from_local = mesh_functions::get_world_from_local(vertex.instance_index);
    out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(expanded_position, 1.0));
    out.clip_position = view_transformations::position_world_to_clip(out.world_position.xyz);

    out.world_normal = mesh_functions::mesh_normal_local_to_world(vertex.normal, vertex.instance_index);

    return out;
}

fn fresnel(normal: vec3<f32>, view: vec3<f32>, power: f32) -> f32 {
    return pow(1.0 - clamp(dot(normalize(normal), normalize(view)), 0.0, 1.0), power);
}

@fragment
fn fragment(input: VertexOutput) -> @location(0) vec4<f32> {
    let flags = pbr_bindings::material.flags;
    let alpha_mode = flags & pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_RESERVED_BITS;
    var color = outline_color;

    if alpha_mode != pbr_types::STANDARD_MATERIAL_FLAGS_ALPHA_MODE_OPAQUE {
        let V = normalize(mesh_view_bindings::view.world_position.xyz - input.world_position.xyz);
        let N = normalize(input.world_normal);

        color *= fresnel(N, V, 3.0);
    }

    return color;
}