import package::render::{
    pbr_lighting::{EnvBRDFApprox, F_AB},
    mesh_view_bindings::lights,
};

/// A precomputed `NdotV` is provided because it is computed regardless,
/// but `world_normal` and the view vector `V` are provided separately for more advanced uses.
/// `specular_color` and `specular_F90` are the specular reflectance at normal incidence and at
/// grazing angles; see `pbr_lighting::specular_F0` and `pbr_lighting::specular_F90`.
fn ambient_light(
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    V: vec3<f32>,
    NdotV: f32,
    diffuse_color: vec3<f32>,
    specular_color: vec3<f32>,
    specular_F90: f32,
    perceptual_roughness: f32,
    occlusion: vec3<f32>,
) -> vec3<f32> {
    let diffuse_ambient = EnvBRDFApprox(diffuse_color, F_AB(1.0, NdotV));
    let specular_F_ab = F_AB(perceptual_roughness, NdotV);
    let specular_ambient = specular_color * specular_F_ab.x + specular_F90 * specular_F_ab.y;

    return (diffuse_ambient + specular_ambient) * lights.ambient_color.rgb * occlusion;
}
