// https://cwyman.org/papers/hpg21_rearchitectingReSTIR.pdf

import bevy_pbr::render::rgb9e5::{vec3_to_rgb9e5_, rgb9e5_to_vec3_};
import bevy_render::utils::{octahedral_encode, octahedral_decode};
import package::scene::sampling::{generate_random_light_sample, light_is_hittable, ResolvedLightSample};
import package::realtime::bindings::{light_tile_samples, light_tile_resolved_samples, view, constants, ResolvedLightSamplePacked};

enable wgpu_ray_query;

@compute @workgroup_size(1024, 1, 1)
fn presample_light_tiles(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(local_invocation_index) sample_index: u32) {
    let tile_id = workgroup_id.x;
    var rng = (tile_id * 0x9E3779B9u) + sample_index + constants.frame_rng;

    let sample = generate_random_light_sample(&rng);

    let i = (tile_id * 1024u) + sample_index;
    light_tile_samples[i] = sample.light_sample;
    light_tile_resolved_samples[i] = pack_resolved_light_sample(sample.resolved_light_sample, light_is_hittable(sample.light_source));
}

fn pack_resolved_light_sample(sample: ResolvedLightSample, brdf_rays_can_hit: bool) -> ResolvedLightSamplePacked {
    let inverse_pdf = sample.inverse_pdf * select(1.0, -1.0, sample.world_position.w == 0.0);
    return ResolvedLightSamplePacked(
        vec4(
            bitcast<vec3<u32>>(sample.world_position.xyz),
            pack2x16unorm(octahedral_encode(sample.world_normal)),
        ),
        vec4(
            vec3_to_rgb9e5_(log2(sample.radiance * view.exposure + 1.0)),
            bitcast<u32>(inverse_pdf),
            pack2x16unorm(octahedral_encode(sample.emission_axis)),
            pack_emission_cone(sample.cone_cos_inner, sample.cone_cos_outer, sample.emission_cosine_exponent)
                | (u32(brdf_rays_can_hit) << 31u),
        ),
    );
}

fn unpack_resolved_light_sample(packed: ResolvedLightSamplePacked, exposure: f32) -> ResolvedLightSample {
    let inverse_pdf = bitcast<f32>(packed.b.y);
    let cone = packed.b.w;
    return ResolvedLightSample(
        vec4(bitcast<vec3<f32>>(packed.a.xyz), select(1.0, 0.0, inverse_pdf < 0.0)),
        octahedral_decode(unpack2x16unorm(packed.a.w)),
        (exp2(rgb9e5_to_vec3_(packed.b.x)) - 1.0) / exposure,
        abs(inverse_pdf),
        octahedral_decode(unpack2x16unorm(packed.b.z)),
        unpack_cone_cos(cone & CONE_COS_MASK),
        unpack_cone_cos((cone >> 15u) & CONE_COS_MASK),
        f32((cone >> 30u) & 1u),
    );
}

const CONE_COS_MASK = 0x7FFFu;
const CONE_COS_MAX = 32767.0;

fn pack_emission_cone(cos_inner: f32, cos_outer: f32, emission_cosine_exponent: f32) -> u32 {
    return pack_cone_cos(cos_inner) | (pack_cone_cos(cos_outer) << 15u) | (u32(emission_cosine_exponent == 1.0) << 30u);
}

fn pack_cone_cos(cos_angle: f32) -> u32 {
    return u32(round(sqrt(saturate((1.0 - cos_angle) * 0.5)) * CONE_COS_MAX));
}

fn unpack_cone_cos(x: u32) -> f32 {
    let sin_half_angle = f32(x) / CONE_COS_MAX;
    return 1.0 - 2.0 * sin_half_angle * sin_half_angle;
}
