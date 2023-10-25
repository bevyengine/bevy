#define_import_path bevy_pbr::ambient

#import bevy_pbr::{
    lighting::{EnvBRDFApprox, F_AB},
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
    specular_color: vec3<f32>,
    perceptual_roughness: f32,
    occlusion: vec3<f32>,
) -> vec3<f32> {
    let diffuse_ambient = EnvBRDFApprox(diffuse_color, F_AB(1.0, NdotV));
    let specular_ambient = EnvBRDFApprox(specular_color, F_AB(perceptual_roughness, NdotV));

    // No real world material has specular values under 0.02, so we use this range as a
    // "pre-baked specular occlusion" that extinguishes the fresnel term, for artistic control.
    // See: https://google.github.io/filament/Filament.html#specularocclusion
    let specular_occlusion = saturate(dot(specular_color, vec3(50.0 * 0.33)));

    return (diffuse_ambient + specular_ambient * specular_occlusion) * lights.ambient_color.rgb * occlusion;
}
