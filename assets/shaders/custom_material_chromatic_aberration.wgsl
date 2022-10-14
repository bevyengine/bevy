#import bevy_pbr::mesh_view_bindings
#import bevy_pbr::mesh_vertex_output
#import bevy_pbr::utils

@group(1) @binding(0)
var texture: texture_2d<f32>;

@group(1) @binding(1)
var our_sampler: sampler;

@fragment
fn fragment(
    mesh: bevy_pbr::mesh_vertex_output::MeshVertexOutput
) -> @location(0) vec4<f32> {
    let view = bevy_pbr::mesh_view_bindings::view;
    // Get screen position with coordinates from 0 to 1
    let uv = bevy_pbr::utils::coords_to_viewport_uv(mesh.clip_position.xy, view.viewport);
    let offset_strength = 0.02;

    // Sample each color channel with an arbitrary shift
    var output_color = vec4<f32>(
        textureSample(texture, our_sampler, uv + vec2<f32>(offset_strength, -offset_strength)).r,
        textureSample(texture, our_sampler, uv + vec2<f32>(-offset_strength, 0.0)).g,
        textureSample(texture, our_sampler, uv + vec2<f32>(0.0, offset_strength)).b,
        1.0
        );

    return output_color;
}
