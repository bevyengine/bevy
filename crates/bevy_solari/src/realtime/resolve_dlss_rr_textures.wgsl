#import bevy_pbr::pbr_deferred_types::unpack_24bit_normal
#import bevy_pbr::utils::octahedral_decode
#import bevy_render::view::View

@group(1) @binding(7) var gbuffer: texture_2d<u32>;
@group(1) @binding(12) var<uniform> view: View;

@group(2) @binding(0) var diffuse_albedo: texture_storage_2d<rgba8unorm, write>;
@group(2) @binding(1) var specular_albedo: texture_storage_2d<rgba8unorm, write>;
@group(2) @binding(2) var normal_roughness: texture_storage_2d<rgba16float, write>;
@group(2) @binding(3) var specular_motion_vectors: texture_storage_2d<rg16float, write>;

@compute @workgroup_size(8, 8, 1)
fn resolve_dlss_rr_textures(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel_id = global_id.xy;
    if any(pixel_id >= vec2u(view.main_pass_viewport.zw)) { return; }

    let gpixel = textureLoad(gbuffer, pixel_id, 0);
    let base_rough = unpack4x8unorm(gpixel.r);
    let base_color = pow(base_rough.rgb, vec3(2.2));
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let perceptual_roughness = base_rough.a;

    textureStore(diffuse_albedo, pixel_id, vec4(base_color, 0.0));
    textureStore(specular_albedo, pixel_id, vec4(0.0)); // TODO
    textureStore(normal_roughness, pixel_id, vec4(world_normal, perceptual_roughness));
    textureStore(specular_motion_vectors, pixel_id, vec4(0.0)); // TODO
}
