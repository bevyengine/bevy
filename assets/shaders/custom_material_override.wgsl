#from bevy_pbr::lighting            import point_light
#from bevy_pbr::mesh_view_types     import PointLight
#from bevy_pbr::mesh_vertex_output  import MeshVertexOutput
#from bevy_pbr::fragment            import fragment

fn quantize_steps() -> f32 {
     return 2.0;
}

override fn ::point_light (
    world_position: vec3<f32>, 
    light: ::PointLight, 
    roughness: f32, 
    NdotV: f32, 
    N: vec3<f32>, 
    V: vec3<f32>,
    R: vec3<f32>, 
    F0: vec3<f32>, 
    diffuseColor: vec3<f32>
) -> vec3<f32> {
    // call original function
    let original = ::point_light(world_position, light, roughness, NdotV, N, V, R, F0, diffuseColor);
    // quantize
    let quantized = vec3<u32>(original * quantize_steps() + 0.5);
    return clamp(vec3<f32>(quantized) / quantize_steps(), vec3<f32>(0.0), vec3<f32>(1.0));
}

@fragment
fn fragment(
    mesh: ::MeshVertexOutput,
    @builtin(front_facing) is_front: bool,
) -> @location(0) vec4<f32> {
    return ::fragment(mesh, is_front);
}
