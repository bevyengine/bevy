#define_import_path bevy_pbr::ambient

#import bevy_pbr::{
    lighting::{F_AB, material_specular_reflectance},
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
    let specular_ambient = material_specular_reflectance(F0_dielectric, F0_metallic, metallic, F_ab);

    // No real world material has specular values under 0.02, so we use this range as a
    // "pre-baked specular occlusion" that extinguishes the fresnel term, for artistic control.
    // See: https://google.github.io/filament/Filament.md.html#specularocclusion
    let specular_occlusion = saturate(dot(mix(F0_dielectric, F0_metallic, metallic), vec3(50.0 * 0.33)));

    return (diffuse_ambient + specular_ambient * specular_occlusion) * lights.ambient_color.rgb * occlusion;
}
