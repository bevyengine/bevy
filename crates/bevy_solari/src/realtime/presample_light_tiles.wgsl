#define_import_path bevy_solari::presample_light_tiles

#import bevy_pbr::rgb9e5::{vec3_to_rgb9e5_, rgb9e5_to_vec3_}
#import bevy_pbr::utils::{rand_u, rand_range_u, octahedral_encode, octahedral_decode}
#import bevy_render::view::View
#import bevy_solari::sampling::{LightSample, ResolvedLightSample, triangle_barycentrics}
#import bevy_solari::scene_bindings::{light_sources, directional_lights, resolve_triangle_data_full, LIGHT_SOURCE_KIND_DIRECTIONAL}

@group(1) @binding(1) var<storage, read_write> light_tile_samples: array<LightSample>;
@group(1) @binding(2) var<storage, read_write> light_tile_resolved_samples: array<ResolvedLightSamplePacked>;
@group(1) @binding(12) var<uniform> view: View;
struct PushConstants { frame_index: u32, reset: u32 }
var<push_constant> constants: PushConstants;

@compute @workgroup_size(1024, 1, 1)
fn presample_light_tiles(@builtin(workgroup_id) workgroup_id: vec3<u32>, @builtin(local_invocation_index) sample_index: u32) {
    let tile_id = workgroup_id.x;
    var rng = (tile_id * 5782582u) + sample_index + constants.frame_index;

    let light_count = arrayLength(&light_sources);
    let light_id = rand_range_u(light_count, &rng);
    let seed = rand_u(&rng);

    let light_source = light_sources[light_id];

    var triangle_id = 0u;
    if light_source.kind != LIGHT_SOURCE_KIND_DIRECTIONAL {
        let triangle_count = light_source.kind >> 1u;
        triangle_id = seed % triangle_count;
    }

    let light_sample = LightSample((light_id << 16u) | triangle_id, seed);

    var resolved_light_sample: ResolvedLightSample;
    if light_source.kind == LIGHT_SOURCE_KIND_DIRECTIONAL {
        // TODO: Add support for DIRECTIONAL_LIGHT_SOFT_SHADOWS
        let directional_light = directional_lights[light_source.id];

        resolved_light_sample = ResolvedLightSample(
            vec4(directional_light.direction_to_light, 0.0),
            -directional_light.direction_to_light,
            directional_light.luminance,
            directional_light.inverse_pdf,
        );
    } else {
        let triangle_count = light_source.kind >> 1u;
        let barycentrics = triangle_barycentrics(seed);
        let triangle_data = resolve_triangle_data_full(light_source.id, triangle_id, barycentrics);

        resolved_light_sample = ResolvedLightSample(
            vec4(triangle_data.world_position, 1.0),
            triangle_data.world_normal,
            triangle_data.material.emissive.rgb,
            f32(triangle_count) * triangle_data.triangle_area,
        );
    }
    resolved_light_sample.inverse_pdf *= f32(light_count);

    let i = (tile_id * 1024u) + sample_index;
    light_tile_samples[i] = light_sample;
    light_tile_resolved_samples[i] = pack_resolved_light_sample(resolved_light_sample);
}

struct ResolvedLightSample {
    world_position: vec4<f32>,
    world_normal: vec3<f32>,
    radiance: vec3<f32>,
    inverse_pdf: f32,
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
