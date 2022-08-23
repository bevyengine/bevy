#import bevy_pbr::lighting
#import bevy_pbr::mesh_view_types
#import bevy_pbr::mesh_vertex_output
#import bevy_pbr::fragment

fn quantize_steps() -> f32 {
     return 2.0;
}

override fn bevy_pbr::lighting::point_light (
    world_position: vec3<f32>, 
    light: bevy_pbr::mesh_view_types::PointLight, 
    roughness: f32, 
    NdotV: f32, 
    N: vec3<f32>, 
    V: vec3<f32>,
    R: vec3<f32>, 
    F0: vec3<f32>, 
    diffuseColor: vec3<f32>
) -> vec3<f32> {
    // call original function
    let original = 
        bevy_pbr::lighting::point_light(world_position, light, roughness, NdotV, N, V, R, F0, diffuseColor);
    // quantize
    let quantized = vec3<u32>(original * quantize_steps() + 0.5);
    return clamp(vec3<f32>(quantized) / quantize_steps(), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fragment(
    mesh: bevy_pbr::mesh_vertex_output::MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
) -> @location(0) vec4<f32> {
    return bevy_pbr::fragment::fragment(mesh, is_front, frag_coord);
}
