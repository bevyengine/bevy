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
    diffuse_occlusion: vec3<f32>,
    specular_occlusion: f32,
) -> vec3<f32> {
    let F_ab_diffuse = F_AB(1.0, NdotV);
    let diffuse_ambient = diffuse_color * (F_ab_diffuse.x + F_ab_diffuse.y) * diffuse_occlusion;
    let Fr = max(vec3(1.0 - perceptual_roughness), specular_color) - specular_color;
    let kS = specular_color + Fr * pow(1.0 - NdotV, 5.0);
    let F_ab_vals = F_AB(perceptual_roughness, NdotV);
    let Ess = F_ab_vals.x + F_ab_vals.y;
    // No real world material has specular values under 0.02, so we use this range as a
    // "pre-baked specular occlusion" that extinguishes the fresnel term, for artistic control.
    // See: https://google.github.io/filament/Filament.html#specularocclusion
    let pre_baked_specular_occlusion = saturate(dot(specular_color, vec3(50.0 * 0.33)));
    let specular_ambient = kS * Ess * pre_baked_specular_occlusion * specular_occlusion;

    return (diffuse_ambient + specular_ambient) * lights.ambient_color.rgb;
}
