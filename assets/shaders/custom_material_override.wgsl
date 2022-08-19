#import bevy_pbr::lighting as Lighting
#import bevy_pbr::mesh_view_types as ViewTypes
#import bevy_pbr::mesh_vertex_output as OutputTypes
#import bevy_pbr::fragment as Pbr

fn quantize_steps() -> f32 {
     return 2.0;
}

override fn Lighting::point_light (
    world_position: vec3<f32>, 
    light: ViewTypes::PointLight, 
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
        Lighting::point_light(world_position, light, roughness, NdotV, N, V, R, F0, diffuseColor);
    // quantize
    let quantized = vec3<u32>(original * quantize_steps() + 0.5);
    return clamp(vec3<f32>(quantized) / quantize_steps(), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fragment(
    mesh: OutputTypes::MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
    @builtin(position) frag_coord: vec4<f32>,
) -> @location(0) vec4<f32> {
    return Pbr::fragment(mesh, is_front, frag_coord);
}
