// https://cwyman.org/papers/hpg21_rearchitectingReSTIR.pdf

#define_import_path bevy_solari::presample_light_tiles

#import bevy_pbr::rgb9e5::{vec3_to_rgb9e5_, rgb9e5_to_vec3_}
#import bevy_pbr::utils::{octahedral_encode, octahedral_decode}
#import bevy_solari::realtime_bindings::{light_tile_samples, view, constants, LightSamplePacked}
#import bevy_solari::sampling::{select_random_light, sample_light, resolve_light_sample, ResolvedLightSample}

@compute @workgroup_size(1024, 1, 1)
fn presample_light_tiles(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(local_invocation_index) sample_index: u32) {
    let tile_id = workgroup_id.x;
    var rng = (tile_id * 5782582u) + sample_index + constants.frame_index;

    let light = select_random_light(&rng);
    let sample = resolve_light_sample(sample_light(&rng, light));

    let i = (tile_id * 1024u) + sample_index;
    light_tile_samples[i] = pack_light_sample(sample);
}

fn pack_light_sample(sample: ResolvedLightSample) -> LightSamplePacked {
    return LightSamplePacked(
        sample.light_id,
        sample.world_position.x,
        sample.world_position.y,
        sample.world_position.z,
        pack2x16unorm(octahedral_encode(sample.world_normal)),
        vec3_to_rgb9e5_(log2(sample.radiance * view.exposure + 1.0)),
        sample.inverse_pdf * select(1.0, -1.0, sample.world_position.w == 0.0),
    );
}

fn unpack_light_sample(packed: LightSamplePacked) -> ResolvedLightSample {
    return ResolvedLightSample(
        packed.light_id,
        vec4(packed.world_position_x, packed.world_position_y, packed.world_position_z, select(1.0, 0.0, packed.inverse_pdf < 0.0)),
        octahedral_decode(unpack2x16unorm(packed.world_normal)),
        (exp2(rgb9e5_to_vec3_(packed.radiance)) - 1.0) / view.exposure,
        abs(packed.inverse_pdf),
    );
}
