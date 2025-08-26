#import bevy_pbr::pbr_functions::calculate_tbn_mikktspace
#import bevy_render::maths::{orthonormalize, PI}
#import bevy_render::view::View
#import bevy_solari::brdf::evaluate_brdf
#import bevy_solari::gbuffer_utils::gpixel_resolve
#import bevy_solari::sampling::{sample_ggx_vndf, ggx_vndf_pdf}
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, ResolvedMaterial, RAY_T_MIN, RAY_T_MAX}
#import bevy_solari::world_cache::{query_world_cache, get_cell_size}

@group(1) @binding(0) var view_output: texture_storage_2d<rgba16float, read_write>;
@group(1) @binding(7) var gbuffer: texture_2d<u32>;
@group(1) @binding(8) var depth_buffer: texture_depth_2d;
@group(1) @binding(12) var<uniform> view: View;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

@compute @workgroup_size(8, 8, 1)
fn specular_trace(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.main_pass_viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.main_pass_viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 {
        return;
    }
    let surface = gpixel_resolve(textureLoad(gbuffer, global_id.xy, 0), depth, global_id.xy, view.main_pass_viewport.zw, view.world_from_clip);
    // if surface.material.roughness > 0.04 { return; } // TODO

    let wo = normalize(view.world_position - surface.world_position);
    let TBN = orthonormalize(surface.world_normal);
    let next_bounce = prepare_next_bounce(wo, TBN, surface.material, &rng);
    var throughput = next_bounce.throughput;
    var wi = next_bounce.wi;
    var ray_origin = surface.world_position;

    loop {
        // Trace ray
        let ray = trace_ray(ray_origin, wi, RAY_T_MIN, RAY_T_MAX, RAY_FLAG_NONE);
        if ray.kind == RAY_QUERY_INTERSECTION_NONE { break; }
        let ray_hit = resolve_ray_hit_full(ray);

        // If cone spread larger than cache cell size, terminate in world cache
        if true { // TODO
            let radiance = query_world_cache(ray_hit.world_position, ray_hit.geometric_world_normal, view.world_position);
            let diffuse_brdf = ray_hit.material.base_color / PI;
            throughput *= radiance * diffuse_brdf;
            break;
        }

        // Prepare next bounce
        let TBN = calculate_tbn_mikktspace(ray_hit.world_normal, ray_hit.world_tangent);
        let next_bounce = prepare_next_bounce(-wi, TBN, surface.material, &rng);
        throughput *= next_bounce.throughput;
        wi = next_bounce.wi;
        ray_origin = ray_hit.world_position;
    }

    var pixel_color = textureLoad(view_output, global_id.xy);
    pixel_color += vec4(throughput * view.exposure, 0.0);
    textureStore(view_output, global_id.xy, pixel_color);
}


struct NextBounce {
    wi: vec3<f32>,
    throughput: vec3<f32>,
}

fn prepare_next_bounce(wo: vec3<f32>, TBN: mat3x3<f32>, material: ResolvedMaterial, rng: ptr<function, u32>) -> NextBounce {
    let T = TBN[0];
    let B = TBN[1];
    let N = TBN[2];

    let wo_tangent = vec3(dot(wo, T), dot(wo, B), dot(wo, N));

    // Sample new ray direction from the GGX BRDF for next bounce
    let wi_tangent = sample_ggx_vndf(wo_tangent, material.roughness, rng);
    let wi = wi_tangent.x * T + wi_tangent.y * B + wi_tangent.z * N;

    // Update throughput for next bounce
    let pdf = ggx_vndf_pdf(wo_tangent, wi_tangent, material.roughness);
    let brdf = evaluate_brdf(N, wo, wi, material); // TODO: Full BRDF or specular-only BRDF?
    let cos_theta = dot(wi, N);
    let throughput = (brdf * cos_theta) / pdf;

    return NextBounce(wi, throughput);
}
