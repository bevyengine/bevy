#import bevy_render::view::View
#import bevy_solari::scene_bindings::
#import bevy_pbr::utils::{PI, rand_f, rand_vec2f}
#import bevy_core_pipeline::tonemapping::tonemapping_luminance

@group(1) @binding(0) var accumulation_texture: texture_storage_2d<rgba32float, read_write>;
@group(1) @binding(1) var view_output: texture_storage_2d<rgba16float, write>;
@group(1) @binding(2) var<uniform> view: View;

@compute @workgroup_size(8, 8, 1)
fn pathtrace(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) {
        return;
    }

    let old_color = textureLoad(accumulation_texture, global_id.xy);

    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    let frame_index = u32(old_color.a) * 5782582u;
    var rng = pixel_index + frame_index;

    let pixel_center = vec2<f32>(global_id.xy) + 0.5;
    let jitter = rand_vec2f(&rng) - 0.5;
    let pixel_uv = (pixel_center + jitter) / view.viewport.zw;
    let pixel_ndc = (pixel_uv * 2.0) - 1.0;
    let primary_ray_target = view.world_from_clip * vec4(pixel_ndc.x, -pixel_ndc.y, 1.0, 1.0);
    var ray_origin = view.world_position;
    var ray_direction = normalize((primary_ray_target.xyz / primary_ray_target.w) - ray_origin);
    var ray_t_min = 0.0;

    var irradiance = vec3(0.0);
    var throughput = vec3(1.0);
    // loop {
    //     let ray_hit = trace_ray(ray_origin, ray_direction, ray_t_min, RAY_T_MAX);
    //     if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
    //         let ray_hit = resolve_ray_hit(ray_hit);

    //         irradiance += ray_hit.material.emissive * throughput;

    //         let cos_theta = dot(ray_hit.world_normal, -ray_direction);
    //         let diffuse_brdf = ray_hit.material.base_color / PI;
    //         let cosine_hemisphere_pdf = cos_theta / PI;
    //         throughput *= (diffuse_brdf * cos_theta) / cosine_hemisphere_pdf;

    //         let p = min(0.95, tonemapping_luminance(throughput));
    //         if rand_f(&rng) > p { break; }
    //         throughput /= p;

    //         ray_origin = ray_hit.world_position;
    //         ray_direction = sample_cosine_hemisphere(ray_hit.world_normal, &rng);
    //         ray_t_min = RAY_T_MIN;
    //     } else { break; }
    // }

    irradiance *= view.exposure;

    let new_color = (irradiance + old_color.a * old_color.rgb) / (old_color.a + 1.0);
    textureStore(accumulation_texture, global_id.xy, vec4(new_color, old_color.a + 1.0));
    textureStore(view_output, global_id.xy, vec4(new_color, 1.0));
}
