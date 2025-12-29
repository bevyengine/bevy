#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::pbr_deferred_types::{unpack_unorm4x8_, unpack_24bit_normal}
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_pbr::rgb9e5::rgb9e5_to_vec3_
#import bevy_pbr::utils::{rand_f, rand_range_u, octahedral_decode, sample_disk}
#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_solari::realtime_bindings::{view_output, gbuffer, depth_buffer, view, previous_view, constants}
#import bevy_solari::scene_bindings::ResolvedMaterial
#import bevy_solari::world_cache::{query_world_cache_lights, evaluate_lighting_from_cache, write_world_cache_light, WORLD_CACHE_CELL_LIFETIME}

@compute @workgroup_size(8, 8, 1)
fn shade(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.main_pass_viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.main_pass_viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        return;
    }

    var material: ResolvedMaterial;
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let wo = normalize(view.world_position - world_position);
    let base_rough = unpack4x8unorm(gpixel.r);
    let props = unpack_unorm4x8_(gpixel.b);

    material.base_color = pow(base_rough.rgb, vec3(2.2));
    material.emissive = rgb9e5_to_vec3_(gpixel.g);
    material.reflectance = vec3(props.r);
    material.perceptual_roughness = base_rough.a;
    material.roughness = clamp(base_rough.a * base_rough.a, 0.001, 1.0);
    material.metallic = props.g;

    let cell = query_world_cache_lights(&rng, world_position, world_normal, view.world_position);
    let direct_lighting = evaluate_lighting_from_cache(&rng, cell, world_position, world_normal, wo, material, view.exposure);
    write_world_cache_light(&rng, direct_lighting, world_position, world_normal, view.world_position, WORLD_CACHE_CELL_LIFETIME, view.exposure);

    let pixel_color = (direct_lighting.radiance * direct_lighting.inverse_pdf + material.emissive) * view.exposure;
    textureStore(view_output, global_id.xy, vec4(pixel_color, 1.0));
}

fn reconstruct_world_position(pixel_id: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_id) + 0.5) / view.main_pass_viewport.zw;
    let xy_ndc = (uv - vec2(0.5)) * vec2(2.0, -2.0);
    let world_pos = view.world_from_clip * vec4(xy_ndc, depth, 1.0);
    return world_pos.xyz / world_pos.w;
}
