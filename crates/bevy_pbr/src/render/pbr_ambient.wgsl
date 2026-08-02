#define_import_path bevy_pbr::ambient

#import bevy_pbr::{
    lighting::{F_AB, material_specular_reflectance, dielectric_specular_occlusion},
    mesh_view_bindings::lights,
}

// A precomputed `NdotV` is provided because it is computed regardless,
// but `world_normal` and the view vector `V` are provided separately for more advanced uses.
fn ambient_light(
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    V: vec3<f32>,
    NdotV: f32,
    diffuse_color: vec3<f32>,
    F0_dielectric: vec3<f32>,
    F0_metallic: vec3<f32>,
    metallic: f32,
    perceptual_roughness: f32,
    occlusion: vec3<f32>,
) -> vec3<f32> {
    let diffuse_ambient = diffuse_color;

    let F_ab = F_AB(perceptual_roughness, NdotV);
    let specular_ambient = material_specular_reflectance(
        F0_dielectric,
        F0_metallic,
        metallic,
        F_ab,
        dielectric_specular_occlusion(F0_dielectric)
    );

    return (diffuse_ambient + specular_ambient) * lights.ambient_color.rgb * occlusion;
}
