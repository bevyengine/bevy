enable wgpu_ray_query;
#define_import_path bevy_solari::resolve_dlss_rr_textures

#import bevy_pbr::lighting::material_specular_reflectance
#import bevy_pbr::pbr_functions::{calculate_diffuse_color, calculate_F0_dielectric}
#import bevy_solari::brdf::F_AB
#import bevy_solari::gbuffer_utils::gpixel_resolve
#import bevy_solari::realtime_bindings::{gbuffer, depth_buffer, motion_vectors, view, diffuse_albedo, specular_albedo, normal_roughness, specular_motion_vectors}

@compute @workgroup_size(8, 8, 1)
fn resolve_dlss_rr_textures(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_id = global_id.xy;
    if any(pixel_id >= vec2u(view.main_pass_viewport.zw)) { return; }

    textureStore(specular_motion_vectors, pixel_id, textureLoad(motion_vectors, pixel_id, 0));

    let depth = textureLoad(depth_buffer, pixel_id, 0);
    if depth == 0.0 {
        textureStore(diffuse_albedo, pixel_id, vec4(0.0));
        textureStore(specular_albedo, pixel_id, vec4(0.5));
        textureStore(normal_roughness, pixel_id, vec4(0.0, 0.0, 1.0, 0.0));
        return;
    }

    let surface = gpixel_resolve(textureLoad(gbuffer, pixel_id, 0), depth, pixel_id, view.main_pass_viewport.zw, view.world_from_clip);
    let wo = normalize(view.world_position - surface.world_position);
    let NdotV = max(dot(surface.world_normal, wo), 0.0001);
    let F_ab = F_AB(surface.material.perceptual_roughness, NdotV);
    let F0_dielectric = calculate_F0_dielectric(vec3(surface.material.reflectance));

    textureStore(diffuse_albedo, pixel_id, vec4(calculate_diffuse_color(surface.material.base_color, surface.material.metallic, 0.0, 0.0, F0_dielectric, F_ab), 0.0));
    textureStore(specular_albedo, pixel_id, vec4(material_specular_reflectance(F0_dielectric, surface.material.base_color, surface.material.metallic, F_ab), 0.0));
    textureStore(normal_roughness, pixel_id, vec4(surface.world_normal, surface.material.perceptual_roughness));
}
