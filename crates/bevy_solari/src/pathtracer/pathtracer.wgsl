#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::utils::{rand_f, rand_vec2f}
#import bevy_render::maths::PI
#import bevy_render::view::View
#import bevy_solari::brdf::evaluate_brdf
#import bevy_solari::sampling::{sample_cosine_hemisphere, sample_ggx_vndf, ggx_vndf_pdf}
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, ResolvedRayHitFull, RAY_T_MIN, RAY_T_MAX}

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
            let wo = -ray_direction;

            // Calculate emissive contribution
            radiance += throughput * ray_hit.material.emissive;

            // Sample new ray direction from the material BRDF for next bounce
            let next_bounce = importance_sample_next_bounce(wo, ray_hit, &rng);
            ray_direction = next_bounce.wi;
            ray_origin = ray_hit.world_position;
            ray_t_min = RAY_T_MIN;

            // Evaluate material BRDF
            let brdf = evaluate_brdf(ray_hit.world_normal, wo, next_bounce.wi, ray_hit.material);

            // Update throughput for next bounce
            let cos_theta = dot(next_bounce.wi, ray_hit.world_normal);
            throughput *= (brdf * cos_theta) / next_bounce.pdf;

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

struct NextBounce {
    wi: vec3<f32>,
    pdf: f32,
}

fn importance_sample_next_bounce(wo: vec3<f32>, ray_hit: ResolvedRayHitFull, rng: ptr<function, u32>) -> NextBounce {
    let diffuse_weight = 1.0 - ray_hit.material.metallic;
    let specular_weight = ray_hit.material.metallic;
    let total_weight = diffuse_weight + specular_weight;

    var wi: vec3<f32>;
    if rand_f(rng) * total_weight < diffuse_weight {
        wi = sample_cosine_hemisphere(ray_hit.world_normal, rng);
    } else {
        wi = sample_ggx_vndf(wo, ray_hit.material.roughness, rng);
    }

    let diffuse_pdf = dot(wi, ray_hit.world_normal) / PI;
    let specular_pdf = ggx_vndf_pdf(wo, wi, ray_hit.world_normal, ray_hit.material.roughness);
    let pdf = ((diffuse_weight * diffuse_pdf) + (specular_weight * specular_pdf)) / total_weight;

    return NextBounce(wi, pdf);
}
