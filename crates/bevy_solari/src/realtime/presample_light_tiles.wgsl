// https://cwyman.org/papers/hpg21_rearchitectingReSTIR.pdf

#define_import_path bevy_solari::presample_light_tiles

#import bevy_pbr::rgb9e5::{vec3_to_rgb9e5_, rgb9e5_to_vec3_}
#import bevy_pbr::utils::{octahedral_encode, octahedral_decode}
#import bevy_render::view::View
#import bevy_solari::sampling::{generate_random_light_sample, LightSample, ResolvedLightSample}

@group(1) @binding(1) var<storage, read_write> light_tile_samples: array<LightSample>;
@group(1) @binding(2) var<storage, read_write> light_tile_resolved_samples: array<ResolvedLightSamplePacked>;
@group(1) @binding(12) var<uniform> view: View;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

@compute @workgroup_size(1024, 1, 1)
fn presample_light_tiles(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(local_invocation_index) sample_index: u32) {
    let tile_id = workgroup_id.x;
    var rng = (tile_id * 5782582u) + sample_index + constants.frame_index;

    let sample = generate_random_light_sample(&rng);

    let i = (tile_id * 1024u) + sample_index;
    light_tile_samples[i] = sample.light_sample;
    light_tile_resolved_samples[i] = pack_resolved_light_sample(sample.resolved_light_sample);
}

struct ResolvedLightSamplePacked {
    world_position_x: f32,
    world_position_y: f32,
    world_position_z: f32,
    world_normal: u32,
    radiance: u32,
    inverse_pdf: f32,
}

fn pack_resolved_light_sample(sample: ResolvedLightSample) -> ResolvedLightSamplePacked {
    return ResolvedLightSamplePacked(
        sample.world_position.x,
        sample.world_position.y,
        sample.world_position.z,
        pack2x16unorm(octahedral_encode(sample.world_normal)),
        vec3_to_rgb9e5_(sample.radiance * view.exposure),
        sample.inverse_pdf * select(1.0, -1.0, sample.world_position.w == 0.0),
    );
}

fn unpack_resolved_light_sample(packed: ResolvedLightSamplePacked, exposure: f32) -> ResolvedLightSample {
    return ResolvedLightSample(
        vec4(packed.world_position_x, packed.world_position_y, packed.world_position_z, select(1.0, 0.0, packed.inverse_pdf < 0.0)),
        octahedral_decode(unpack2x16unorm(packed.world_normal)),
        rgb9e5_to_vec3_(packed.radiance) / exposure,
        abs(packed.inverse_pdf),
    );
}
