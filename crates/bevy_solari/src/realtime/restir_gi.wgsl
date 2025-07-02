// https://intro-to-restir.cwyman.org/presentations/2023ReSTIR_Course_Notes.pdf

#import bevy_core_pipeline::tonemapping::tonemapping_luminance as luminance
#import bevy_pbr::pbr_deferred_types::unpack_24bit_normal
#import bevy_pbr::prepass_bindings::PreviousViewUniforms
#import bevy_pbr::rgb9e5::rgb9e5_to_vec3_
#import bevy_pbr::utils::{rand_f, octahedral_decode}
#import bevy_render::maths::{PI, PI_2}
#import bevy_render::view::View
#import bevy_solari::sampling::{sample_uniform_hemisphere, sample_random_light}
#import bevy_solari::scene_bindings::{trace_ray, resolve_ray_hit_full, RAY_T_MIN, RAY_T_MAX}

@group(1) @binding(0) var view_output: texture_storage_2d<rgba16float, read_write>;
// @group(1) @binding(1) var<storage, read_write> di_reservoirs_a: array<Reservoir>;
// @group(1) @binding(2) var<storage, read_write> di_reservoirs_b: array<Reservoir>;
@group(1) @binding(3) var gbuffer: texture_2d<u32>;
@group(1) @binding(4) var depth_buffer: texture_depth_2d;
@group(1) @binding(5) var motion_vectors: texture_2d<f32>;
@group(1) @binding(6) var previous_gbuffer: texture_2d<u32>;
@group(1) @binding(7) var previous_depth_buffer: texture_depth_2d;
@group(1) @binding(8) var<uniform> view: View;
@group(1) @binding(9) var<uniform> previous_view: PreviousViewUniforms;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

@compute @workgroup_size(8, 8, 1)
fn initial_and_temporal(@builtin(global_invocation_id) global_id: vec3<u32>) {
    if any(global_id.xy >= vec2u(view.viewport.zw)) { return; }

    let pixel_index = global_id.x + global_id.y * u32(view.viewport.z);
    var rng = pixel_index + constants.frame_index;

    let depth = textureLoad(depth_buffer, global_id.xy, 0);
    if depth == 0.0 { return; }
    let gpixel = textureLoad(gbuffer, global_id.xy, 0);
    let world_position = reconstruct_world_position(global_id.xy, depth);
    let world_normal = octahedral_decode(unpack_24bit_normal(gpixel.a));
    let base_color = pow(unpack4x8unorm(gpixel.r).rgb, vec3(2.2));
    let diffuse_brdf = base_color / PI;

    let ray_direction = sample_uniform_hemisphere(world_normal, &rng);
    let ray_hit = trace_ray(world_position, ray_direction, RAY_T_MIN, RAY_T_MAX, RAY_FLAG_NONE);
    if ray_hit.kind == RAY_QUERY_INTERSECTION_NONE { return; }
    let sample_point = resolve_ray_hit_full(ray_hit);
    if all(sample_point.material.emissive != vec3(0.0)) { return; }
    let sample_point_diffuse_brdf = sample_point.material.base_color / PI;
    let radiance = sample_random_light(sample_point.world_position, sample_point.world_normal, &rng);

    let cos_theta = dot(ray_direction, world_normal);
    let inverse_uniform_hemisphere_pdf = PI_2;
    let contribution = (radiance * sample_point_diffuse_brdf * diffuse_brdf * cos_theta * inverse_uniform_hemisphere_pdf);

    var pixel_color = textureLoad(view_output, global_id.xy);
    pixel_color += vec4(contribution * view.exposure, 0.0);
    textureStore(view_output, global_id.xy, pixel_color);
}

fn reconstruct_world_position(pixel_id: vec2<u32>, depth: f32) -> vec3<f32> {
    let uv = (vec2<f32>(pixel_id) + 0.5) / view.viewport.zw;
    let xy_ndc = (uv - vec2(0.5)) * vec2(2.0, -2.0);
    let world_pos = view.world_from_clip * vec4(xy_ndc, depth, 1.0);
    return world_pos.xyz / world_pos.w;
}
