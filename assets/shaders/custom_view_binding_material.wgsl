#import bevy_pbr::mesh_view_bindings

struct CustomMaterial {
    color: vec4<f32>,
};

@group(1) @binding(0)
var<uniform> material: CustomMaterial;

@fragment
fn fragment(
    #import bevy_pbr::mesh_vertex_output
) -> @location(0) vec4<f32> {
    return material.color * (0.5 + 0.5 * sin(custom_view_binding.time));
}
