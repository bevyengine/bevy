#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::{rand_f, rand_vec2f}
#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_solari::sampling::{sample_random_light, sample_cosine_hemisphere}
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, RAY_T_MIN, RAY_T_MAX}

@group(1) @binding(0) var accumulation_texture: texture_storage_2d<rgba32float, read_write>;
@group(1) @binding(1) var view_output: texture_storage_2d<rgba16float, write>;
@group(1) @binding(2) var<uniform> view: View;

@compute @workgroup_size(8, 8, 1)
fn pathtrace(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) {
        return;
    }

    let old_color = textureLoad(accumulation_texture, global_id.xy);

    // Setup RNG
    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    let frame_index = u32(old_color.a) * 5782582u;
    var rng = pixel_index + frame_index;

    // Shoot the first ray from the camera
    let pixel_center = vec2<f32>(global_id.xy) + 0.5;
    let jitter = rand_vec2f(&rng) - 0.5;
    let pixel_uv = (pixel_center + jitter) / view.viewport.zw;
    let pixel_ndc = (pixel_uv * 2.0) - 1.0;
    let primary_ray_target = view.world_from_clip * vec4(pixel_ndc.x, -pixel_ndc.y, 1.0, 1.0);
    var ray_origin = view.world_position;
    var ray_direction = normalize((primary_ray_target.xyz / primary_ray_target.w) - ray_origin);
    var ray_t_min = 0.0;

    // Path trace
    var radiance = vec3(0.0);
    var throughput = vec3(1.0);
    loop {
        let ray_hit = trace_ray(ray_origin, ray_direction, ray_t_min, RAY_T_MAX, RAY_FLAG_NONE);
        if ray_hit.kind != RAY_QUERY_INTERSECTION_NONE {
            let ray_hit = resolve_ray_hit_full(ray_hit);

            // Evaluate material BRDF
            let diffuse_brdf = ray_hit.material.base_color / PI;

            // Use emissive only on the first ray (coming from the camera)
            if ray_t_min == 0.0 { radiance = ray_hit.material.emissive; }

            // Sample direct lighting
            let direct_lighting = sample_random_light(ray_hit.world_position, ray_hit.world_normal, &rng);
            radiance += throughput * diffuse_brdf * direct_lighting.radiance * direct_lighting.inverse_pdf;

            // Sample new ray direction from the material BRDF for next bounce
            ray_direction = sample_cosine_hemisphere(ray_hit.world_normal, &rng);

            // Update other variables for next bounce
            ray_origin = ray_hit.world_position;
            ray_t_min = RAY_T_MIN;

            // Update throughput for next bounce
            let cos_theta = dot(-ray_direction, ray_hit.world_normal);
            let cosine_hemisphere_pdf = cos_theta / PI; // Weight for the next bounce because we importance sampled the diffuse BRDF for the next ray direction
            throughput *= (diffuse_brdf * cos_theta) / cosine_hemisphere_pdf;

            // Russian roulette for early termination
            let p = luminance(throughput);
            if rand_f(&rng) > p { break; }
            throughput /= p;
        } else { break; }
    }

    // Camera exposure
    radiance *= view.exposure;

    // Accumulation over time via running average
    let new_color = mix(old_color.rgb, radiance, 1.0 / (old_color.a + 1.0));
    textureStore(accumulation_texture, global_id.xy, vec4(new_color, old_color.a + 1.0));
    textureStore(view_output, global_id.xy, vec4(new_color, 1.0));
}
