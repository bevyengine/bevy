#define_import_path quantize_lights

#import bevy_pbr::lighting
#import bevy_pbr::mesh_view_types

struct QuantizeStepsStruct {
    steps: f32,
}

@group(0) @binding(auto)
var<uniform> quantize_steps_struct: QuantizeStepsStruct;

fn quantize_steps() -> f32 {
     return quantize_steps_struct.steps;
}

override fn bevy_pbr::lighting::point_light(
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
    let original = bevy_pbr::lighting::point_light(world_position, light, roughness, NdotV, N, V, R, F0, diffuseColor);
    // quantize
    let quantized = vec3<u32>(original * quantize_steps() + 0.5);
    return clamp(vec3<f32>(quantized) / quantize_steps(), vec3<f32>(0.0), vec3<f32>(1.0));
}

override fn bevy_pbr::lighting::directional_light(
    light: bevy_pbr::mesh_view_types::DirectionalLight, 
    roughness: f32, 
    NdotV: f32, 
    normal: vec3<f32>, 
    view: vec3<f32>, 
    R: vec3<f32>, 
    F0: vec3<f32>, 
    diffuseColor: vec3<f32>
) -> vec3<f32> {
    // call original function
    let original = bevy_pbr::lighting::directional_light(light, roughness, NdotV, normal, view, R, F0, diffuseColor);
    // quantize
    let quantized = vec3<u32>(original * quantize_steps() + 0.5);
    return clamp(vec3<f32>(quantized) / quantize_steps(), vec3<f32>(0.0), vec3<f32>(1.0));
}